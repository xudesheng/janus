# Janus: What and Why

Janus 研究的是 OpenTelemetry 处理管道之后的系统应该如何设计。

OpenTelemetry 已经很好地标准化了遥测数据的产生与交换：API、SDK 行为、上下文传播、语义约定、数据模型、OTLP，以及 Collector 管道。它给行业提供了一套共同契约，让 traces、metrics、logs、resources 和相关上下文能够以一致的方式被创建、处理和导出。

Janus 从下一个边界开始。

它的问题不是“如何重新定义遥测采集”，而是：

> 如果 observability backend 的第一消费者不是人类 dashboard 用户，而是 AI Agent，那么后端应该如何存储、组织、摘要和检索遥测数据？

## 背景

OpenTelemetry 的边界主要在 instrumentation、data model、exchange protocol 和 pipeline 层。

OTel 规范覆盖 API、SDK、数据规范、语义约定、协议、资源、traces、metrics、logs 等内容。OTLP 定义 telemetry data 在 sources、collectors 这类 intermediate nodes，以及 telemetry backends 之间传输时的 encoding、transport 和 delivery mechanism。OpenTelemetry Collector 则提供 vendor-agnostic 的 receive、process、export 管道，通过 receivers、processors、exporters、connectors 和 pipelines 处理数据。

这是 OTel 正确且重要的边界。

但 backend 边界是另一件事。数据从 SDK exporter 或 Collector exporter 出来之后，OpenTelemetry 并不规定后端必须如何持久化、索引、压缩、查询、关联、摘要、授权、保留或展示这些数据。现有 observability 系统通常会根据自己的目标选择实现方式：高吞吐写入、高压缩率、时间序列聚合、日志搜索、trace lookup、alert rule evaluation、dashboard rendering 等。

Janus 把这个 backend 空间视为新的设计问题。

公开参考：

- OpenTelemetry Specification: https://opentelemetry.io/docs/specs/otel/
- OpenTelemetry overview: https://opentelemetry.io/docs/specs/otel/overview/
- OTLP specification: https://opentelemetry.io/docs/specs/otlp/
- OpenTelemetry Collector: https://opentelemetry.io/docs/collector/
- Collector architecture: https://opentelemetry.io/docs/collector/architecture/
- Semantic conventions: https://opentelemetry.io/docs/concepts/semantic-conventions/

## 核心转变

传统 observability backend 主要服务人类操作者。它们的典型产品表面是 dashboard、query console、alert rule、trace waterfall、log search page。这些界面很有价值，但它们也会把存储模型引向人类已经知道如何提出的问题：

- 过去 15 分钟这个 service 的 p95 latency 是多少？
- 这个 route 出现了多少 5xx？
- 查找 `level=error` 的日志。
- 打开这个 trace ID。
- 比较这个 dashboard panel 和昨天的差异。

AI Agent 的工作流不同。Agent 通常不是为了渲染 dashboard，而是为了调查、形成假设、收集证据、修正解释，然后建议或执行下一步动作。

所以有用的 backend primitive 不只是“对原始 telemetry 做快速查询”，而是：

> 快速组装和解释相关 operational context。

Janus 应该优先支持这类问题：

- 系统变得不健康前后发生了什么变化？
- 哪些 services、deployments、nodes、routes、tenants、dependencies 被卷入了问题？
- 哪些 traces、metric anomalies、log patterns、change events 是最强证据？
- 这次问题更像是 deploy、traffic shift、dependency degradation、resource exhaustion、retry storm、schema change、configuration change，还是 downstream outage？
- Agent 在接下来一分钟内真正需要的最小证据集合是什么？
- 如果当前假设不够强，Agent 下一步应该检查什么？

这会改变 backend 的设计优先级。

Janus 应该在热数据路径上优先考虑 Agent 的工作质量，即使这会牺牲一部分存储效率或增加预处理成本。长期冷数据可以在之后更激进地压缩、采样、摘要或归档，因为旧数据通常对即时 Agent 响应不再同等重要。

## 产品假设

Janus 是一个面向 AI Agent 的、兼容 OpenTelemetry 数据形态的 observability backend。

它应该接收标准 telemetry，尽量保留和利用 OTel data model 的兼容性，然后把数据重组为更适合 Agent 调查的结构。

Janus 首先不是：

- dashboard database；
- time-series database；
- log search engine；
- trace viewer；
- RCA Agent。

Janus 首先应该是：

> context and evidence system。

更进一步，Janus 可以被理解为：

> operational evidence compiler。

它把散乱、异构、时间敏感的 signals 编译成 Agent 可以消费的 evidence IR：结构化、带 provenance、可排序、可裁剪、可回溯、能表达不确定性的调查中间表示。

Agent 仍然负责推理、沟通和行动；Janus 负责让推理被正确证据约束。

## 设计优先级

1. 热路径上，Agent 响应质量优先于存储效率。

   对近期数据，Janus 应该保留并派生足够上下文，让 Agent 能做出高质量判断。这可能需要冗余索引、entity graph、window summary、representative samples、anomaly segments、precomputed correlations 等结构。

2. 秒级到分钟级响应是可以接受的。

   Janus 不需要把每个调查问题都压到毫秒级。它应该支持快速的交互式 Agent 工作：短 planning loop、evidence gathering、hypothesis update、follow-up query。只要证据质量明显更好，几秒到少量分钟的响应时间可以接受。

3. Context 是一等存储对象。

   原始 spans、metric points、log records 很重要，但不够。Janus 应该存储派生上下文：entities、relationships、change events、anomaly windows、pattern clusters、summaries、evidence bundles。

4. 近期数据对 AI 工作更重要。

   Agent 最关键的价值通常发生在 incident、regression、deploy、traffic spike、用户影响症状附近。hot layer 应该丰富、冗余、易检索。cold layer 可以更便宜、更粗糙。

5. 证据必须可回溯。

   Summaries 和 embeddings 有用，但不能替代 source evidence。Agent 得出的结论必须能够回到其背后的 spans、logs、metrics、change records。

6. Backend 应该支持 investigation，而不只是 retrieval。

   Query API 不应只返回 rows 或 time buckets。它应该暴露更接近调查过程的能力：寻找相关异常、构建 incident timeline、比较健康和异常窗口、追踪 dependency impact、对 suspected causes 排序。

7. Janus 应该降低 false causality。

   对运维场景来说，一个叙事顺畅但错误的解释，比“证据不足”更危险。Janus 应该让错误因果链更难形成，而不是让 Agent 更自信地给出单一答案。

## 非目标

Janus 不重新定义 OpenTelemetry instrumentation。

应用开发者不应该为了 Janus 采用一套新的 telemetry API。OTel API、SDK、semantic conventions、resource model、context propagation、OTLP 和 Collector 生态应该继续作为上游契约。

Janus 初期不追求 human dashboard parity。

Dashboard 以后可以存在，但不应该支配存储模型。Dashboard 常常需要固定面板、规则时间桶、已知 label；Agent 需要 context、evidence、causality、contrast 和 next-best inspection。

Janus 不把“长期保存每一条原始 telemetry”作为主要价值。

永远保存所有字节既昂贵，也经常不能帮助诊断。更关键的问题是：什么表示方式最能帮助 Agent 诊断当前和近期问题？当热窗口过去之后，什么低成本表示仍然有调查价值？

Janus 不隐藏不确定性。

生产系统里的因果关系很少是绝对确定的。Backend 应该保留 confidence、alternative hypotheses、counter-evidence、missing data、time alignment quality，而不是把每次调查都压成一个确定的 root cause。

Janus 不应该变成“又一个 AI SRE agent”。

行业里已经有很多 RCA agent、triage assistant、mitigation copilot 和多智能体系统。Janus 更有价值的位置是 substrate：让这些 Agent 更容易拿到正确证据。

## 数据模型方向

AI-first backend 应该围绕 operational meaning 组织数据，而不只是围绕 signal type 组织数据。

原始 OTel signals 仍然重要：

- traces and spans；
- metric streams and data points；
- log records and events；
- resources；
- instrumentation scopes；
- attributes；
- trace context 和可用的 baggage；
- 未来可能包括 profiles。

Janus 应该增加 Agent 可以直接使用的派生对象：

- Entity：service、route、operation、host、container、pod、deployment、database、queue、external API、tenant、region、feature flag、build、model。
- Relationship：calls、depends-on、runs-on、owns、deployed-as、emits、retries、fans-out-to、reads-from、writes-to、shares-resource-with。
- Change event：deploy、rollback、config update、feature flag change、traffic shift、scaling event、schema migration、dependency version change、infrastructure event。
- Anomaly window：一个有边界的时间区间，在其中一个或多个 signals 偏离预期行为。
- Pattern cluster：按结构和含义聚合的 logs、errors、span events、exceptions、status changes。
- Evidence bundle：支持或削弱某个假设的一小组 traces、logs、metric segments、entities、changes、comparisons。
- Incident timeline：按时间排列的 symptoms、changes、propagation effects、mitigations、recovery signs。
- Summary：对 time window、entity 或 investigation step 的压缩描述，并且链接回 source evidence。

这层派生模型不是 OTel 的替代品，而是建立在 OTel-compatible telemetry 之上的 backend semantic layer。

## Evidence IR

Janus 不应该只给 Agent 暴露一组底层查询工具。更好的边界是定义稳定的 Evidence IR。

一个 evidence item 至少应该包含：

- `claim`：这条证据支持或削弱的陈述；
- `kind`：metric anomaly、trace exemplar、log cluster、change event、dependency edge、profile hotspot、previous incident、counter-evidence 等；
- `time_window`：证据有效的时间范围；
- `entities`：相关 service、route、host、pod、deployment、tenant、dependency；
- `source_refs`：可回溯到 raw spans、logs、metrics、profiles、change records 或外部 backend 的指针；
- `strength`：证据强度，不等同于因果置信度；
- `direction`：supports、weakens、contradicts、neutral；
- `freshness`：证据是否仍在变化；
- `missing_data`：这条证据依赖但当前缺失的数据；
- `token_cost`：放入 Agent 上下文的大致成本；
- `privacy_scope`：权限、租户、敏感字段裁剪信息。

Evidence IR 能避免一个常见失败：把 summary 当事实。Summary 只能是 evidence item 的一种展示或压缩形式，不能切断 source refs。

## Token Budget 是查询约束

传统 observability 查询通常按 time range、label selector、aggregation、limit 约束结果。

Agent 查询还必须有一个新的硬约束：

> 在给定 token budget 内，返回最大诊断价值的证据集合。

这不是简单的结果截断。Janus 需要在检索时就理解 token budget：

- 优先返回能区分假设的证据，而不是信息量最大的证据；
- 优先返回覆盖不同 failure modes 的证据，而不是同质日志样本；
- 同时返回支持证据和反证；
- 对高成本原始材料先返回 summary + source refs；
- 在证据不足时返回 missing data，而不是编造完整解释。

因此，`get_evidence_bundle` 不应只是 `LIMIT N`。它应该接收 question、hypothesis、time window、entities、max items、max tokens、counter-evidence requirement、raw refs requirement、freshness requirement 和 privacy scope。

## 存储分层

Janus 应该采用 time-aware storage layers，并让不同层有不同优先级。

### Hot Layer

Hot layer 保存近期 telemetry 和派生上下文。它的第一目标是 Agent usefulness。

保留周期可以是分钟到小时，也可以根据成本和部署规模扩展到几天。

Hot layer 应该支持：

- full 或 high-fidelity 的近期 traces、logs、metrics、resources、events；
- entity-resolution indexes；
- dependency 和 runtime topology graphs；
- trace-to-log-to-metric correlation；
- recent change indexes；
- anomaly windows；
- log 和 error pattern clusters；
- representative trace/log samples；
- 必要时使用 embeddings 支持 retrieval；
- 链接到精确证据的短摘要；
- investigation sessions 和 hypothesis state。

Hot layer 可以冗余。只要能帮助 Agent 更快拿到正确上下文，同一事实可以用多种形式存储。

### Warm Layer

Warm layer 保存较少原始细节和更多 curated context。

保留周期可以是天到周。

Warm layer 应该保留：

- entity histories；
- service dependency evolution；
- incident summaries；
- representative examples；
- anomaly summaries；
- high-value traces；
- log pattern histories；
- deploy/config timelines；
- 带 anomaly annotations 的 metric rollups。

Warm layer 应该回答：

- 这以前发生过吗？
- 同样的 route、dependency、tenant、region、deployment pattern 是否出现在过去 incident 中？
- 上次 mitigation 是什么？
- 这个异常对该 service 是常见还是罕见？

### Cold Layer

Cold layer 以成本为主。

保留周期可以是月到年。

在并存架构下，Janus 的 cold layer 不一定要长期保存全部原始 telemetry。更现实的定义是：

> Cold Janus = durable understanding + backlinks, not full raw telemetry retention。

也就是保留 summaries、incident memory、entity history、evidence metadata、source pointers；全量原始数据可以留在现有后端或 object store。

## Agent-Oriented Query Surface

传统 backend 暴露的是围绕 logs、traces、metrics 的 query language。

Janus 应该暴露 investigation primitives。

候选 primitives：

- `get_evidence_bundle`：为某个 question 或 hypothesis 返回有界、排序后的 source-backed evidence。
- `build_timeline`：返回一个窗口内按时间排序的 symptoms、changes、propagation effects、recovery markers。
- `expand_entity_context`：返回某个 entity 的 dependencies、owners、recent changes、current health、related incidents。
- `find_related_anomalies`：按 entity、dependency、time、attribute、topology 返回相关 anomalies。
- `compare_windows`：跨 metrics、logs、traces、changes 比较健康窗口和异常窗口。
- `rank_suspected_causes`：基于 time alignment、blast radius、dependency direction、change proximity、error signatures、counter-evidence 对候选原因排序。
- `suggest_next_checks`：当证据不完整时，告诉 Agent 下一步应该检查什么。

这些 API 背后可以是传统数据库、搜索系统、vector index、graph store、object storage，或者自定义 Rust 服务。关键不是底层技术，而是 contract：backend 返回 structured context 和 inspectable evidence，而不只是 rows。

这些 primitives 也很适合作为 MCP tools 暴露给外部 Agent。

## 从相关工作中提炼出的约束

LLM for AIOps、AI SRE、incident management、log analysis 和 RCA benchmark 的相关工作共同指向几个约束。

第一，Janus 不应该把“生成 root cause”作为 API contract。

RCA 可以是 evaluation task，也可以是 Agent 的输出，但 Janus 应该对 evidence quality 负责，而不是对单一根因叙事负责。

第二，false causality 是核心安全问题。

Agent 可能从不完整或巧合相关的证据中生成流畅但错误的解释。Janus 应该显式暴露 time alignment、dependency direction、counter-evidence、missing data 和 entity confidence。

第三，query recommendation 属于 backend 能力。

Agent 经常不知道下一步该查什么。`suggest_next_checks` 不应该只是 prompt trick，而应该基于 Janus 存储的 topology、recent changes、available signals 和 known gaps。

第四，logs 应该变成 pattern objects。

Raw log search 对 Agent 来说通常太吵。更有用的对象是 log pattern、frequency trend、first seen / last seen、affected entities、representative exemplars、related traces 和 source refs。

第五，eval harness 必须尽早出现。

Janus 的核心命题必须可测量：同一个 Agent，在同一个 incident 上，使用 Janus Evidence IR 是否比直接访问 raw backend 更快、更准、更少 false causality、更省 token、更可审计？

## 从现有 AI Native APM 中得到的提醒

已经有项目在快速推进 AI Native OpenTelemetry APM：OTLP ingest、trace assembly、trace-derived metrics、service topology、Doris/columnar storage、AI experts、MCP integration、Docker/K8s 快速部署。

这说明需求是真实的，产品入口也会很快变得拥挤。

但这也帮助 Janus 收敛边界。

Janus 不应该把第一目标放在“重建一个完整 APM UI”上。APM UI、dashboard、trace viewer、metric panel、service list 和 alarm rule 都是成熟且竞争激烈的产品面。

Janus 更应该专注于相邻但更底层的东西：

- Evidence IR；
- provenance；
- counter-evidence；
- token-budget-aware retrieval；
- entity-resolution confidence；
- false-causality eval；
- agent-oriented MCP tools；
- hot/warm operational memory。

换句话说，AI Native APM 抢的是产品入口；Janus 应该占的是 evidence substrate。

## 架构含义

Janus 至少应该包含这些概念组件：

- OTel ingestion：接收 OTLP 或 Collector-exported telemetry。
- Raw telemetry store：保留近期 source data 和被选择进入长期存储的数据。
- Change ingestor：捕获 deploys、config changes、feature flags、scaling events、CI/CD events、infrastructure changes。
- Entity resolver：把 resources、attributes、spans、logs、metrics 映射到稳定 operational entities，并输出置信度。
- Relationship builder：构建 dependency、runtime、ownership、deployment graphs。
- Anomaly detector：跨 signals 找到有边界的异常窗口。
- Pattern clusterer：聚合相关 logs、exceptions、span statuses、events。
- Evidence compiler：生成 Evidence IR、evidence bundles、timelines、source refs、counter-evidence 和 missing-data records。
- Investigation API：暴露 Agent-first workflows。
- MCP interface：把 investigation primitives 暴露给外部 Agent。
- Retention and compaction pipeline：把数据从 hot 移到 warm 再到 cold，同时保留有调查价值的记忆。
- Eval harness：用 incident corpus 评估 Agent outcome。

这些组件一开始不需要都是独立服务。它们首先是设计职责。小实现可以从少量 tables、indexes、background jobs 和 MCP tools 开始，只要职责边界保持清楚。

## 并存，而非替代

Janus 不应该要求用户迁移掉已有 observability backend。

OpenTelemetry Collector 原生支持 fan-out，同一份 telemetry 可以同时导向现有后端和 Janus。Janus 可以只消费近期窗口，构建 Agent 所需的语义工作记忆。

这带来几个好处：

- 接入风险低：增加一个 exporter target，而不是替换现有后端；
- 成本边界清楚：昂贵的 derived context 只活在 hot/warm layer；
- cold layer 可以瘦身：长期 raw bytes 留给已有后端或 object store；
- 证据仍可回溯：Janus 保存 source refs 和 backlinks；
- 市场定位清楚：Janus 是 Agent substrate，不是 dashboard backend 替代品。

## 评估标准

Janus 应该用 Agent outcomes 来评估，而不只看数据库指标。

关键问题：

- Agent 能否更快定位可疑 entity？
- Agent 能否生成有用 incident timeline？
- Agent 能否拿到简洁证据，而不是淹没在原始 logs 中？
- Agent 能否区分 correlation 和 likely causation？
- Agent 能否暴露 counter-evidence？
- Agent 能否说明缺了什么数据？
- Agent 能否把当前 incident 和过去相似 incident 对比？
- Agent 推荐的下一步诊断动作是否合理？

特别要测：

- suspicious-entity accuracy；
- useful timeline quality；
- false-causality rate；
- time-to-useful-hypothesis；
- token cost；
- missing-data awareness；
- human-rated evidence quality；
- auditability。

数据库指标仍然重要：

- ingestion throughput；
- query latency；
- storage cost；
- compaction cost；
- backfill behavior；
- availability；
- correctness。

但在 Janus 中，这些是约束条件。产品目标是更好的 Agent investigation。

## 初始赌注

Janus 的第一个有价值版本不应该试图重建完整 observability backend。

最小可验证赌注是：

1. 接收 OTel-shaped traces、logs、metrics、resources 和 change events。
2. 为近期时间窗口构建 hot operational context store。
3. 跨 signals 解析 entities 和 relationships。
4. 检测或导入 anomaly windows。
5. 聚合相关 errors 和 log patterns。
6. 定义 Evidence IR。
7. 暴露 `get_evidence_bundle`、`build_timeline`、`expand_entity_context`、`suggest_next_checks`。
8. 把旧数据压缩成 summaries、representative examples 和 source backlinks。
9. 建立小型 incident eval corpus，对比 raw backend access 与 Janus evidence access。

这版 MVP 不需要证明 Janus 能自动 RCA。它只需要证明一件事：

> 在同样 Agent、同样 incident、同样时间限制下，Janus 能把更少、更准、更可审计的证据放进 Agent 的上下文。

如果这条路径成立，Janus 给 AI Agent 提供的就不只是 dashboard，而是一个结构化、可回溯、时间敏感的系统运行理解层。
