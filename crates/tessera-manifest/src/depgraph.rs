//! 依赖图（设计书 §3.3、spec `plugin-dependency-graph`）。
//!
//! 手写 Tarjan 强连通分量 + Kahn 拓扑排序（design.md D5：算法只有两个且都
//! 需要定制输出——环路径要从 SCC 还原，通用图库反而要多写代码）。图的
//! 规模是插件数量（几十到几百），性能无关紧要；节点集合用 `BTreeMap`
//! 保证输出顺序确定。
//!
//! 依赖不等于调用权（spec「声明依赖不等同于获得调用权」）：本模块只产出
//! 顺序与失败集合，`command.invoke` 权限的检查归权限装配。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use semver::{Version, VersionReq};
use tessera_error::codes;
use tessera_error::{ErrorCode, TesseraError};

use crate::ids::PluginId;

/// 图节点：一个已 `Validated` 插件的依赖信息。
#[derive(Debug, Clone)]
pub struct DepNode {
    pub id: PluginId,
    pub version: Version,
    /// 依赖声明：目标插件 id → 要求的 SemVer 区间。
    pub dependencies: BTreeMap<PluginId, VersionReq>,
}

/// 依赖图：以插件为节点、`dependencies` 声明为有向边（A 依赖 B → 边 A→B）。
#[derive(Debug, Clone, Default)]
pub struct DepGraph {
    nodes: BTreeMap<PluginId, DepNode>,
}

impl DepGraph {
    pub fn from_nodes(nodes: Vec<DepNode>) -> Self {
        Self {
            nodes: nodes.into_iter().map(|n| (n.id.clone(), n)).collect(),
        }
    }

    pub fn node(&self, id: &PluginId) -> Option<&DepNode> {
        self.nodes.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &PluginId> {
        self.nodes.keys()
    }

    /// 直接依赖 `id` 的插件集合（反向边查询，级联传播用）。
    fn dependents_of(&self, id: &PluginId) -> Vec<&PluginId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.dependencies.contains_key(id))
            .map(|(k, _)| k)
            .collect()
    }

    /// §3.3 依赖解析全流程：存在性/版本 → 环检测 → 级联失败 → 拓扑排序。
    pub fn resolve(&self) -> DepResolution {
        // 1. 存在性与版本（含自依赖 = 自环，直接按 E_DEPS_CYCLE 报）。
        //    一个节点可能同时有缺失与版本问题：缺失优先定码（装都装不上，
        //    版本无从谈起），两类问题都写进信息一次报出（D2）。
        let mut failures: BTreeMap<PluginId, TesseraError> = BTreeMap::new();
        for node in self.nodes.values() {
            let mut missing = Vec::new();
            let mut version = Vec::new();
            let mut self_dep = false;
            for (dep_id, req) in &node.dependencies {
                if dep_id == &node.id {
                    self_dep = true;
                    continue;
                }
                match self.nodes.get(dep_id) {
                    None => missing.push(format!("依赖的插件 `{dep_id}` 未安装（要求 {req}）")),
                    Some(dep) => {
                        if !req.matches(&dep.version) {
                            version.push(format!(
                                "依赖 `{dep_id}` 的版本不满足：要求 {req}，实际已安装 {}",
                                dep.version
                            ));
                        }
                    }
                }
            }
            let problems: Vec<String> = missing.iter().chain(version.iter()).cloned().collect();
            if self_dep {
                failures.insert(
                    node.id.clone(),
                    TesseraError::new(
                        ErrorCode::Deps(codes::DepsCode::Cycle),
                        format!(
                            "插件 `{}` 依赖自身（自环）：{} → {}",
                            node.id, node.id, node.id
                        ),
                    ),
                );
            } else if !problems.is_empty() {
                let code = if missing.is_empty() {
                    codes::DepsCode::Version
                } else {
                    codes::DepsCode::Missing
                };
                failures.insert(
                    node.id.clone(),
                    TesseraError::new(ErrorCode::Deps(code), problems.join("；")),
                );
            }
        }

        // 2. 环检测（Tarjan，只在未因存在性/版本失败的节点间进行）
        let active: BTreeSet<&PluginId> = self
            .nodes
            .keys()
            .filter(|id| !failures.contains_key(*id))
            .collect();
        for scc in self.tarjan_sccs(&active) {
            if scc.len() > 1 {
                let cycle = self.recover_cycle_path(&scc);
                let path = cycle
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" → ");
                for id in &scc {
                    failures.insert(
                        id.clone(),
                        TesseraError::new(
                            ErrorCode::Deps(codes::DepsCode::Cycle),
                            format!("依赖环：{path}"),
                        ),
                    );
                }
            }
        }

        // 3. 级联失败：直接/间接依赖失败节点的全部下游 → E_DEPS_UPSTREAM_FAILED（根因传递）
        let cascaded = propagate_upstream_failures(self, &failures);
        let failures: BTreeMap<PluginId, TesseraError> = failures
            .into_iter()
            .chain(cascaded)
            .collect::<BTreeMap<_, _>>();

        // 4. 拓扑排序（Kahn，仅在成功节点子图上）
        let succeeded: BTreeSet<&PluginId> = self
            .nodes
            .keys()
            .filter(|id| !failures.contains_key(*id))
            .collect();
        let load_order = self.kahn_order(&succeeded);

        DepResolution {
            load_order,
            failures,
        }
    }

    /// Tarjan 强连通分量（迭代实现，避免深图递归栈溢出）。
    /// 只遍历 `active` 内的节点与边。
    fn tarjan_sccs(&self, active: &BTreeSet<&PluginId>) -> Vec<Vec<PluginId>> {
        #[derive(Default)]
        struct NodeState {
            index: Option<u32>,
            lowlink: u32,
            on_stack: bool,
        }
        let mut states: BTreeMap<&PluginId, NodeState> = BTreeMap::new();
        let mut stack: Vec<&PluginId> = Vec::new();
        let mut sccs = Vec::new();
        let mut next_index = 0u32;

        // 显式调用栈的帧：(节点, 待访问的邻居迭代器)
        let mut call_stack: Vec<(&PluginId, Vec<&PluginId>)> = Vec::new();

        for start in active.iter().copied() {
            if states.contains_key(start) {
                continue;
            }
            call_stack.push((start, self.neighbors(start, active)));
            states.entry(start).or_default().index = Some(next_index);
            states.get_mut(start).unwrap().lowlink = next_index;
            next_index += 1;
            stack.push(start);
            states.get_mut(start).unwrap().on_stack = true;

            while let Some((v, neighbors)) = call_stack.last_mut() {
                if let Some(w) = neighbors.pop() {
                    if !states.contains_key(w) {
                        states.entry(w).or_default().index = Some(next_index);
                        states.get_mut(w).unwrap().lowlink = next_index;
                        next_index += 1;
                        stack.push(w);
                        states.get_mut(w).unwrap().on_stack = true;
                        let w_neighbors = self.neighbors(w, active);
                        call_stack.push((w, w_neighbors));
                    } else if states[&w].on_stack {
                        // w 在栈上：lowlink[v] = min(lowlink[v], index[w])
                        let wi = states[&w].index.unwrap();
                        if wi < states[v].lowlink {
                            states.get_mut(v).unwrap().lowlink = wi;
                        }
                    }
                } else {
                    let (v, _) = call_stack.pop().unwrap();
                    let vi = states[&v].index.unwrap();
                    if states[&v].lowlink == vi {
                        let mut scc = Vec::new();
                        while let Some(w) = stack.pop() {
                            states.get_mut(&w).unwrap().on_stack = false;
                            scc.push((*w).clone());
                            if w == v {
                                break;
                            }
                        }
                        sccs.push(scc);
                    }
                    if let Some((parent, _)) = call_stack.last_mut() {
                        let vl = states[&v].lowlink;
                        if vl < states[parent].lowlink {
                            states.get_mut(parent).unwrap().lowlink = vl;
                        }
                    }
                }
            }
        }
        sccs
    }

    /// 节点在 `active` 子图内的邻居（依赖边目标），顺序确定。
    fn neighbors<'s>(
        &'s self,
        id: &'s PluginId,
        active: &'s BTreeSet<&'s PluginId>,
    ) -> Vec<&'s PluginId> {
        match self.nodes.get(id) {
            Some(node) => node
                .dependencies
                .keys()
                .filter(|dep| active.contains(dep))
                .collect(),
            None => Vec::new(),
        }
    }

    /// 从 SCC 成员集合还原一条完整环路径（含回到起点的收尾），如 `[A, B, C, A]`。
    /// 自环（成员依赖自身）返回 `[A, A]`。
    fn recover_cycle_path<'a>(&'a self, scc: &'a [PluginId]) -> Vec<PluginId> {
        let members: BTreeSet<&'a PluginId> = scc.iter().collect();
        let start = members.iter().next().copied().expect("SCC 非空");

        // 自环直接短路
        if let Some(node) = self.nodes.get(start)
            && node.dependencies.contains_key(start)
        {
            return vec![start.clone(), start.clone()];
        }

        // DFS：只走 SCC 内的边，找一条从 start 回到 start 的简单路径
        let mut path: Vec<&PluginId> = vec![start];
        let mut on_path: BTreeSet<&PluginId> = BTreeSet::from([start]);
        if let Some(cycle) = self.dfs_cycle(start, start, &members, &mut path, &mut on_path) {
            let mut result: Vec<PluginId> = cycle.into_iter().cloned().collect();
            result.push(start.clone());
            result
        } else {
            // Tarjan 已保证 size>1 的 SCC 内每个节点都在某个环上，此分支不可达
            scc.to_vec()
        }
    }

    fn dfs_cycle<'a>(
        &'a self,
        current: &'a PluginId,
        start: &'a PluginId,
        members: &'a BTreeSet<&'a PluginId>,
        path: &mut Vec<&'a PluginId>,
        on_path: &mut BTreeSet<&'a PluginId>,
    ) -> Option<Vec<&'a PluginId>> {
        for next in self.neighbors(current, members) {
            if next == start {
                return Some(path.clone());
            }
            if on_path.contains(next) {
                continue;
            }
            path.push(next);
            on_path.insert(next);
            if let Some(found) = self.dfs_cycle(next, start, members, path, on_path) {
                return Some(found);
            }
            path.pop();
            on_path.remove(next);
        }
        None
    }

    /// Kahn 拓扑排序（仅在 `succeeded` 子图上）：任何插件排在它的全部依赖之后。
    fn kahn_order(&self, succeeded: &BTreeSet<&PluginId>) -> Vec<PluginId> {
        // remaining_deps[id] = id 在成功子图内尚未出队的依赖数
        let mut remaining_deps: BTreeMap<&PluginId, usize> =
            succeeded.iter().map(|id| (*id, 0usize)).collect();
        for id in succeeded.iter().copied() {
            if let Some(node) = self.nodes.get(id) {
                let count = node
                    .dependencies
                    .keys()
                    .filter(|dep| remaining_deps.contains_key(*dep))
                    .count();
                *remaining_deps.get_mut(id).unwrap() = count;
            }
        }

        let mut ready: BTreeSet<&PluginId> = remaining_deps
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut order = Vec::with_capacity(succeeded.len());
        while let Some(u) = ready.iter().next().copied() {
            ready.remove(u);
            order.push((*u).clone());
            // u 已出队：所有依赖 u 的节点剩余依赖数 -1
            for dependent in self.dependents_of(u) {
                if let Some(d) = remaining_deps.get_mut(dependent) {
                    *d -= 1;
                    if *d == 0 {
                        ready.insert(dependent);
                    }
                }
            }
        }
        debug_assert_eq!(
            order.len(),
            succeeded.len(),
            "成功子图无环，拓扑序必须覆盖全部节点"
        );
        order
    }
}

/// 依赖解析结果。
#[derive(Debug, Clone, Default)]
pub struct DepResolution {
    /// 加载顺序（拓扑序，被依赖者在前）。停用顺序 = 其逆序。
    load_order: Vec<PluginId>,
    /// 失败表：插件 → 错误（`E_DEPS_MISSING` / `E_DEPS_VERSION` / `E_DEPS_CYCLE` /
    /// `E_DEPS_UPSTREAM_FAILED`）。成功插件不在表中。
    pub failures: BTreeMap<PluginId, TesseraError>,
}

impl DepResolution {
    /// 加载顺序：任何插件被激活之前，它的全部直接与间接依赖都已激活。
    pub fn load_order(&self) -> &[PluginId] {
        &self.load_order
    }

    /// 停用顺序 = 加载顺序的逆序（§3.3 第 6 步）：停用某插件之前，
    /// 所有依赖它的插件都已先行停用。
    pub fn shutdown_order(&self) -> Vec<PluginId> {
        self.load_order.iter().rev().cloned().collect()
    }
}

/// 级联失败传播（§3.3 第 5 步）：给定根因失败集合，把全部直接与间接
/// 依赖失败插件的下游标记为 `E_DEPS_UPSTREAM_FAILED`。
///
/// 错误信息**指明根因插件标识**（spec「级联失败指向根因」）：根因沿链
/// 传递——B 根因失败、A 依赖 B、C 依赖 A 时，A 与 C 的错误都指向 B，
/// 用户可见的失败呈现只需针对 B 报告一次（呈现归 UI 层，本函数提供
/// 足够的信息）。
///
/// 返回值只含被级联的下游节点；根因自身的错误原样保留在入参中，由调用方
/// 合并（`resolve` 内部即如此使用）。
pub fn propagate_upstream_failures(
    graph: &DepGraph,
    root_failures: &BTreeMap<PluginId, TesseraError>,
) -> BTreeMap<PluginId, TesseraError> {
    // 队列元素：(当前已失败节点, 根因插件 id)
    let mut queue: VecDeque<(&PluginId, &PluginId)> =
        root_failures.keys().map(|id| (id, id)).collect();
    let mut failed: BTreeSet<&PluginId> = root_failures.keys().collect();
    let mut cascaded: BTreeMap<PluginId, TesseraError> = BTreeMap::new();

    while let Some((node, root)) = queue.pop_front() {
        for dependent in graph.dependents_of(node) {
            if failed.contains(dependent) {
                continue;
            }
            failed.insert(dependent);
            cascaded.insert(
                dependent.clone(),
                TesseraError::new(
                    ErrorCode::Deps(codes::DepsCode::UpstreamFailed),
                    format!("因依赖的插件 `{root}` 失败而无法加载（根因：{root}）"),
                ),
            );
            queue.push_back((dependent, root));
        }
    }
    cascaded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, version: &str, deps: &[(&str, &str)]) -> DepNode {
        let id = PluginId::parse(id).unwrap();
        DepNode {
            id,
            version: Version::parse(version).unwrap(),
            dependencies: deps
                .iter()
                .map(|(dep, req)| {
                    (
                        PluginId::parse(dep).unwrap(),
                        VersionReq::parse(req).unwrap(),
                    )
                })
                .collect(),
        }
    }

    /// 5.1 验证：依赖不存在返回 E_DEPS_MISSING，信息指出缺失插件。
    #[test]
    fn missing_dependency_fails_requester() {
        let graph = DepGraph::from_nodes(vec![node("com.a", "1.0.0", &[("com.b", "^2.0.0")])]);
        let res = graph.resolve();
        let a = PluginId::parse("com.a").unwrap();
        let err = res.failures.get(&a).expect("A 必须失败");
        assert_eq!(err.code.to_string(), "E_DEPS_MISSING");
        assert!(err.user_message.contains("com.b"), "{}", err.user_message);
    }

    /// 5.1 验证：版本不满足返回 E_DEPS_VERSION，信息同时含要求区间与实际版本。
    #[test]
    fn version_mismatch_fails_requester() {
        let graph = DepGraph::from_nodes(vec![
            node("com.a", "1.0.0", &[("com.b", "^2.0.0")]),
            node("com.b", "1.5.0", &[]),
        ]);
        let res = graph.resolve();
        let a = PluginId::parse("com.a").unwrap();
        let b = PluginId::parse("com.b").unwrap();
        let err = res.failures.get(&a).expect("A 必须失败");
        assert_eq!(err.code.to_string(), "E_DEPS_VERSION");
        let msg = &err.user_message;
        assert!(msg.contains("^2.0.0") && msg.contains("1.5.0"), "{msg}");
        assert!(!res.failures.contains_key(&b), "B 本身没有失败");
    }

    /// 5.1 验证：自依赖被拒绝（自环，E_DEPS_CYCLE）。
    #[test]
    fn self_dependency_rejected() {
        let graph = DepGraph::from_nodes(vec![node("com.a", "1.0.0", &[("com.a", "^1.0.0")])]);
        let res = graph.resolve();
        let a = PluginId::parse("com.a").unwrap();
        let err = res.failures.get(&a).unwrap();
        assert_eq!(err.code.to_string(), "E_DEPS_CYCLE");
    }

    /// 5.2 验证：A→B→C→A 三插件环全部失败，错误信息含完整环路径。
    #[test]
    fn three_plugin_cycle_reported_with_full_path() {
        let graph = DepGraph::from_nodes(vec![
            node("com.a", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.b", "1.0.0", &[("com.c", "^1.0.0")]),
            node("com.c", "1.0.0", &[("com.a", "^1.0.0")]),
        ]);
        let res = graph.resolve();
        for id in ["com.a", "com.b", "com.c"] {
            let pid = PluginId::parse(id).unwrap();
            let err = res
                .failures
                .get(&pid)
                .unwrap_or_else(|| panic!("{id} 必须失败"));
            assert_eq!(err.code.to_string(), "E_DEPS_CYCLE", "{id}");
        }
        // 错误信息含 A → B → C → A 形式的完整路径（以 → 连接、首尾相同）
        let err = res
            .failures
            .get(&PluginId::parse("com.a").unwrap())
            .unwrap();
        let msg = &err.user_message;
        assert!(msg.contains("com.a → com.b → com.c → com.a"), "{msg}");
    }

    /// 5.3 验证：环外插件不受影响——只有环上插件失败。
    #[test]
    fn plugins_outside_cycle_unaffected() {
        let graph = DepGraph::from_nodes(vec![
            node("com.a", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.b", "1.0.0", &[("com.a", "^1.0.0")]),
            node("com.standalone", "1.0.0", &[]),
            node("com.leaf", "1.0.0", &[("com.standalone", "^1.0.0")]),
        ]);
        let res = graph.resolve();
        for id in ["com.a", "com.b"] {
            assert!(
                res.failures.contains_key(&PluginId::parse(id).unwrap()),
                "{id} 应在环上失败"
            );
        }
        for id in ["com.standalone", "com.leaf"] {
            assert!(
                !res.failures.contains_key(&PluginId::parse(id).unwrap()),
                "{id} 不应受环影响"
            );
        }
        // 环外节点照常出现在加载序中，且依赖序正确
        let order = res.load_order();
        let sa = order
            .iter()
            .position(|id| id.as_str() == "com.standalone")
            .unwrap();
        let leaf = order
            .iter()
            .position(|id| id.as_str() == "com.leaf")
            .unwrap();
        assert!(sa < leaf, "standalone 必须先于依赖它的 leaf 激活");
    }

    /// 5.4 验证：A 依赖 B 时加载序中 B 在 A 前、停用序中 A 在 B 前。
    #[test]
    fn load_and_shutdown_order_are_topological() {
        let graph = DepGraph::from_nodes(vec![
            node("com.a", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.b", "1.0.0", &[]),
        ]);
        let res = graph.resolve();
        assert!(res.failures.is_empty());

        let load = res.load_order();
        let b = load.iter().position(|id| id.as_str() == "com.b").unwrap();
        let a = load.iter().position(|id| id.as_str() == "com.a").unwrap();
        assert!(b < a, "B 必须先于 A 激活：{load:?}");

        let shutdown = res.shutdown_order();
        let a2 = shutdown
            .iter()
            .position(|id| id.as_str() == "com.a")
            .unwrap();
        let b2 = shutdown
            .iter()
            .position(|id| id.as_str() == "com.b")
            .unwrap();
        assert!(a2 < b2, "A 必须先于 B 停用：{shutdown:?}");
    }

    /// 5.5 验证：B 失败、A 依赖 B、C 依赖 A 时，A 与 C 的错误都指向根因 B。
    #[test]
    fn cascade_failures_point_to_root_cause() {
        let graph = DepGraph::from_nodes(vec![
            node("com.a", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.b", "1.0.0", &[("com.missing", "^1.0.0")]),
            node("com.c", "1.0.0", &[("com.a", "^1.0.0")]),
        ]);
        let res = graph.resolve();
        let a = PluginId::parse("com.a").unwrap();
        let c = PluginId::parse("com.c").unwrap();
        let b = PluginId::parse("com.b").unwrap();

        let err_a = res.failures.get(&a).expect("A 级联失败");
        let err_c = res.failures.get(&c).expect("C 级联失败");
        assert_eq!(err_a.code.to_string(), "E_DEPS_UPSTREAM_FAILED");
        assert_eq!(err_c.code.to_string(), "E_DEPS_UPSTREAM_FAILED");
        // 两者的错误信息都指向根因 B（而不是中间的 A）
        assert!(
            err_a.user_message.contains("com.b") && err_c.user_message.contains("com.b"),
            "A: {}; C: {}",
            err_a.user_message,
            err_c.user_message
        );
        // 根因 B 自己报的是缺失依赖
        assert_eq!(
            res.failures.get(&b).unwrap().code.to_string(),
            "E_DEPS_MISSING"
        );
        // 级联节点不进加载序
        assert!(res.load_order().is_empty());
    }

    /// `propagate_upstream_failures` 独立语义：根因错误可以是任意域
    /// （如激活失败），下游统一标 UPSTREAM_FAILED 且根因传递。
    #[test]
    fn propagate_marks_all_downstream_with_root() {
        let graph = DepGraph::from_nodes(vec![
            node("com.b", "1.0.0", &[]),
            node("com.a", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.c", "1.0.0", &[("com.a", "^1.0.0")]),
            node("com.d", "1.0.0", &[("com.b", "^1.0.0")]),
            node("com.unrelated", "1.0.0", &[]),
        ]);
        let b = PluginId::parse("com.b").unwrap();
        let mut roots = BTreeMap::new();
        roots.insert(
            b.clone(),
            TesseraError::new(
                ErrorCode::Activate(tessera_error::codes::ActivateCode::ReturnedErr),
                "B 激活失败",
            ),
        );
        let cascaded = propagate_upstream_failures(&graph, &roots);

        for id in ["com.a", "com.c", "com.d"] {
            let pid = PluginId::parse(id).unwrap();
            let err = cascaded
                .get(&pid)
                .unwrap_or_else(|| panic!("{id} 应被级联"));
            assert_eq!(err.code.to_string(), "E_DEPS_UPSTREAM_FAILED");
            assert!(err.user_message.contains("com.b"), "{}", err.user_message);
        }
        assert!(!cascaded.contains_key(&PluginId::parse("com.unrelated").unwrap()));
        assert!(!cascaded.contains_key(&b), "根因不在级联表中");
    }

    /// 多根因时不重复标记：已被一个根因波及的节点不再被第二个根因覆盖。
    #[test]
    fn multiple_roots_mark_each_node_once() {
        let graph = DepGraph::from_nodes(vec![
            node("com.b1", "1.0.0", &[]),
            node("com.b2", "1.0.0", &[]),
            node(
                "com.a",
                "1.0.0",
                &[("com.b1", "^1.0.0"), ("com.b2", "^1.0.0")],
            ),
        ]);
        let mut roots = BTreeMap::new();
        for id in ["com.b1", "com.b2"] {
            roots.insert(
                PluginId::parse(id).unwrap(),
                TesseraError::new(ErrorCode::Deps(codes::DepsCode::Missing), "失败"),
            );
        }
        let cascaded = propagate_upstream_failures(&graph, &roots);
        let a = cascaded
            .get(&PluginId::parse("com.a").unwrap())
            .expect("A 被级联");
        assert_eq!(a.code.to_string(), "E_DEPS_UPSTREAM_FAILED");
        // 根因指向前者或后者皆可，但必须是二者之一且只报一个
        assert!(a.user_message.contains("com.b1") || a.user_message.contains("com.b2"));
    }

    /// resolve 集成：多种失败并存（缺失 + 版本 + 环），各自的失败码正确。
    #[test]
    fn resolve_mixes_failure_kinds() {
        let graph = DepGraph::from_nodes(vec![
            node("com.missing-req", "1.0.0", &[("com.nope", "^1.0.0")]),
            node("com.ver-req", "1.0.0", &[("com.old", "^2.0.0")]),
            node("com.old", "1.0.0", &[]),
            node("com.x", "1.0.0", &[("com.y", "^1.0.0")]),
            node("com.y", "1.0.0", &[("com.x", "^1.0.0")]),
            node("com.ok", "1.0.0", &[("com.old", "^1.0.0")]),
        ]);
        let res = graph.resolve();
        assert_eq!(
            res.failures
                .get(&PluginId::parse("com.missing-req").unwrap())
                .unwrap()
                .code
                .to_string(),
            "E_DEPS_MISSING"
        );
        assert_eq!(
            res.failures
                .get(&PluginId::parse("com.ver-req").unwrap())
                .unwrap()
                .code
                .to_string(),
            "E_DEPS_VERSION"
        );
        for id in ["com.x", "com.y"] {
            assert_eq!(
                res.failures
                    .get(&PluginId::parse(id).unwrap())
                    .unwrap()
                    .code
                    .to_string(),
                "E_DEPS_CYCLE"
            );
        }
        // com.ok 依赖的 com.old 版本满足（^1.0.0 匹配 1.0.0），正常加载
        assert!(
            !res.failures
                .contains_key(&PluginId::parse("com.ok").unwrap())
        );
        assert_eq!(res.load_order().len(), 2); // com.ok + com.old
    }
}
