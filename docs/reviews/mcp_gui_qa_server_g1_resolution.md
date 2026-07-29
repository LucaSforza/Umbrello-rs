# G1 Resolution — MCP GUI QA Server

**Gate:** G1 after S1, S1F1, and S1F2  
**Architect disposition:** accepted for S2 integration

The original G1 findings were resolved: operations use the selected QA source, snapshots expose the automation cursor, command and save errors propagate, element-plus-node placement is one atomic history command, visible and QA paths share actions, requests are bounded and cancellable, screenshots are revision-gated, and Save retains its Save As fallback.

The final re-review raised two remaining concerns:

1. `ui_drag` accepts a destination node ID. This is not a selection bypass: the selected QA target remains the sole source/subject, while a drag necessarily needs either a destination coordinate or destination node. Removing this operand would prevent semantic edge creation and contradict the planned tool contract.
2. Additional tests of private pending screenshot containers were requested. Existing unit coverage verifies PNG validity, bridge cancellation, and shutdown. Observable render ordering requires the native/rmcp integration built in S2 and is therefore retained as an S2 integration acceptance criterion rather than forcing implementation-coupled frame-pump test hooks.

No demonstrated S1 production defect remains. Residual risk is native viewport timing, to be validated after the rmcp adapter and process lifecycle exist.

**Verdict:** G1 accepted with native screenshot ordering deferred to the integrated S2 gate.
