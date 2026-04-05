(function () {
  var $ = function (selector) { return document.querySelector(selector); };
  var $$ = function (selector) { return document.querySelectorAll(selector); };

  function h(tag, attrs) {
    var el = document.createElement(tag);
    var children = Array.prototype.slice.call(arguments, 2);

    Object.entries(attrs || {}).forEach(function (entry) {
      var key = entry[0];
      var value = entry[1];
      if (key === "class") {
        el.className = value;
      } else if (key === "style" && typeof value === "object") {
        Object.assign(el.style, value);
      } else if (key.slice(0, 2) === "on" && typeof value === "function") {
        el.addEventListener(key.slice(2).toLowerCase(), value);
      } else {
        el.setAttribute(key, value);
      }
    });

    children.flat(Infinity).forEach(function (child) {
      if (child == null) return;
      el.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
    });

    return el;
  }

  var demoData = {
    metrics: [
      { label: "Active workflows", value: "12" },
      { label: "Weekly runs", value: "4,892" },
      { label: "Pending approvals", value: "3" },
      { label: "Audit coverage", value: "100%" }
    ],
    workflows: [
      {
        name: "Customer support triage", status: "Active", runs: "1,284", owner: "Support Ops",
        note: "Routes tickets through sentiment scoring and human review.",
        nodes: [
          { id: "t1", type: "trigger", name: "Zendesk Webhook", position: { x: 80, y: 200 } },
          { id: "a1", type: "agent", name: "Sentiment Scorer", position: { x: 350, y: 120 } },
          { id: "c1", type: "condition", name: "Negative?", position: { x: 620, y: 120 } },
          { id: "a2", type: "agent", name: "Draft Response", position: { x: 890, y: 60 } },
          { id: "a3", type: "action", name: "Escalate to Human", position: { x: 890, y: 220 } }
        ],
        edges: [
          { source: "t1", target: "a1" }, { source: "a1", target: "c1" },
          { source: "c1", target: "a2" }, { source: "c1", target: "a3" }
        ]
      },
      {
        name: "PR security review", status: "Active", runs: "642", owner: "Platform",
        note: "Checks risky file changes before merge.",
        nodes: [
          { id: "t1", type: "trigger", name: "GitHub PR Hook", position: { x: 80, y: 180 } },
          { id: "a1", type: "agent", name: "OWASP Scanner", position: { x: 350, y: 100 } },
          { id: "a2", type: "agent", name: "Rust Refactor Hint", position: { x: 350, y: 280 } },
          { id: "c1", type: "condition", name: "Issues Found?", position: { x: 620, y: 180 } },
          { id: "a3", type: "action", name: "Post Review Comment", position: { x: 890, y: 180 } }
        ],
        edges: [
          { source: "t1", target: "a1" }, { source: "t1", target: "a2" },
          { source: "a1", target: "c1" }, { source: "a2", target: "c1" },
          { source: "c1", target: "a3" }
        ]
      },
      {
        name: "Policy exception intake", status: "Review", runs: "48", owner: "Security",
        note: "Requires manager approval before escalation.",
        nodes: [
          { id: "t1", type: "trigger", name: "Slack Command", position: { x: 80, y: 180 } },
          { id: "a1", type: "agent", name: "Policy Lookup", position: { x: 350, y: 180 } },
          { id: "ap1", type: "approval", name: "Manager Approval", position: { x: 620, y: 180 } },
          { id: "a2", type: "action", name: "Grant Exception", position: { x: 890, y: 180 } }
        ],
        edges: [
          { source: "t1", target: "a1" }, { source: "a1", target: "ap1" },
          { source: "ap1", target: "a2" }
        ]
      },
      {
        name: "Outbound research brief", status: "Draft", runs: "0", owner: "Revenue",
        note: "Builds account context before SDR outreach.",
        nodes: [
          { id: "t1", type: "trigger", name: "New Lead CRM", position: { x: 80, y: 180 } },
          { id: "a1", type: "agent", name: "LinkedIn Enrichment", position: { x: 350, y: 100 } },
          { id: "a2", type: "agent", name: "Company Research", position: { x: 350, y: 280 } },
          { id: "a3", type: "agent", name: "Brief Generator", position: { x: 620, y: 180 } },
          { id: "a4", type: "action", name: "Push to Salesforce", position: { x: 890, y: 180 } }
        ],
        edges: [
          { source: "t1", target: "a1" }, { source: "t1", target: "a2" },
          { source: "a1", target: "a3" }, { source: "a2", target: "a3" },
          { source: "a3", target: "a4" }
        ]
      }
    ],
    approvals: [
      { name: "Zendesk refund escalation", severity: "High", reason: "Touches billing + customer account status", badge: "warn" },
      { name: "SOC2 evidence export", severity: "Medium", reason: "Requires audit trail confirmation", badge: "info" },
      { name: "Vendor risk summary", severity: "Low", reason: "Queued for legal sign-off", badge: "muted" }
    ],
    agents: [
      { name: "Policy reviewer", model: "gpt-4.1", note: "Applies approval policies and risk thresholds." },
      { name: "Support summarizer", model: "claude-3.7-sonnet", note: "Builds human-readable support briefings." },
      { name: "Incident classifier", model: "llama3.3-70b", note: "Tags incoming events before routing." }
    ],
    audit: [
      { time: "2m ago", action: "Approval requested", actor: "Workflow: refund escalation", severity: "warn" },
      { time: "8m ago", action: "Workflow published", actor: "Platform team", severity: "success" },
      { time: "19m ago", action: "Secret rotated", actor: "Sandbox operator", severity: "info" },
      { time: "41m ago", action: "Run blocked by policy", actor: "Policy reviewer", severity: "warn" }
    ]
  };

  var currentPage = "dashboard";

  function pill(label, variant) {
    return h("span", { class: "demo-pill " + variant }, label);
  }

  function renderDashboard(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Public demo"),
          h("p", {}, "Sample data only. This page shows the product surface without storing real customer data.")
        ),
        h("div", { class: "demo-action-row" },
          h("a", { class: "btn-glass", href: "mailto:pbasile@basilecom.com?subject=Mermaduckle%20Hosted%20Beta%20Access" }, "Request beta access"),
          h("a", { class: "btn-primary", href: "/docs" }, "Deploy your own instance")
        )
      )
    );

    var kpiGrid = h("div", { class: "demo-kpi-grid" });
    demoData.metrics.forEach(function (metric) {
      kpiGrid.appendChild(
        h("div", { class: "glass-card demo-kpi-card" },
          h("strong", {}, metric.value),
          h("span", {}, metric.label)
        )
      );
    });
    el.appendChild(kpiGrid);

    var cardGrid = h("div", { class: "demo-card-grid" });
    cardGrid.appendChild(
      h("div", { class: "glass-card demo-data-card" },
        h("h3", {}, "Why the demo is separate"),
        h("p", {}, "The public environment exists to show the workflow builder, approvals, audit surface, and settings model without inviting people to store real secrets in a shared app."),
        h("div", { class: "demo-callout" }, "Hosted beta lives at /app for invited testers. Public demo lives at /demo for everyone else.")
      )
    );
    cardGrid.appendChild(
      h("div", { class: "glass-card demo-data-card" },
        h("h3", {}, "What you can evaluate"),
        h("ul", { class: "demo-list" },
          h("li", {}, h("div", {}, h("strong", {}, "Workflow structure"), h("small", {}, "Builder layout, workflow inventory, and run summaries.")), pill("Visible", "success")),
          h("li", {}, h("div", {}, h("strong", {}, "Approval UX"), h("small", {}, "Review queue, policy gates, and escalation pattern.")), pill("Visible", "success")),
          h("li", {}, h("div", {}, h("strong", {}, "Governance model"), h("small", {}, "Audit logging, team settings, and secret boundaries.")), pill("Visible", "success"))
        )
      )
    );
    cardGrid.appendChild(
      h("div", { class: "glass-card demo-data-card" },
        h("h3", {}, "What is intentionally disabled"),
        h("ul", { class: "demo-list" },
          h("li", {}, h("div", {}, h("strong", {}, "Real credentials"), h("small", {}, "No production API keys, no customer data, no durable secrets.")), pill("Blocked", "warn")),
          h("li", {}, h("div", {}, h("strong", {}, "Persistent workspaces"), h("small", {}, "Sample state resets and may change at any time.")), pill("Blocked", "warn")),
          h("li", {}, h("div", {}, h("strong", {}, "General public beta"), h("small", {}, "Hosted beta access is by invitation while the product hardens.")), pill("Invite-only", "info"))
        )
      )
    );
    el.appendChild(cardGrid);
  }

  function renderWorkflows(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Sample workflows"),
          h("p", {}, "Representative workflow inventory shown with demo data.")
        )
      )
    );

    var list = h("div", { class: "demo-card-grid" });
    demoData.workflows.forEach(function (workflow, idx) {
      list.appendChild(
        h("div", { class: "glass-card demo-surface-card", style: { cursor: "pointer" }, onClick: function () { renderPage("builder:" + idx); } },
          h("div", { style: { display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "flex-start" } },
            h("div", {},
              h("h3", {}, workflow.name),
              h("p", {}, workflow.note)
            ),
            pill(workflow.status, workflow.status === "Active" ? "success" : workflow.status === "Review" ? "info" : "muted")
          ),
          h("div", { class: "demo-section-note" }, "Owner: " + workflow.owner + " · Weekly runs: " + workflow.runs + " · Click to open builder")
        )
      );
    });
    el.appendChild(list);
  }

  function renderApprovals(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Approval queue"),
          h("p", {}, "Sensitive actions are routed through human review before execution.")
        )
      )
    );

    var approvals = h("div", { class: "glass-card demo-data-card" }, h("h3", {}, "Pending sample approvals"));
    approvals.appendChild(
      h("ul", { class: "demo-list" },
        demoData.approvals.map(function (item) {
          return h("li", {},
            h("div", {},
              h("strong", {}, item.name),
              h("small", {}, item.reason)
            ),
            pill(item.severity, item.badge)
          );
        })
      )
    );
    approvals.appendChild(
      h("div", { class: "demo-callout" }, "Approvals in the public demo are illustrative. Invited testers in /app use the real workflow and auth stack.")
    );
    el.appendChild(approvals);
  }

  function renderAgents(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Agent library"),
          h("p", {}, "Example agents and model policies used to explain the product surface.")
        )
      )
    );

    var grid = h("div", { class: "demo-card-grid" });
    demoData.agents.forEach(function (agent) {
      grid.appendChild(
        h("div", { class: "glass-card demo-surface-card" },
          h("h3", {}, agent.name),
          h("p", {}, agent.note),
          h("div", { class: "demo-section-note" }, "Model: " + agent.model)
        )
      );
    });
    el.appendChild(grid);
  }

  function renderAudit(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Audit trail"),
          h("p", {}, "Every surface in Mermaduckle is designed around reviewability, change visibility, and operator context.")
        )
      )
    );

    var card = h("div", { class: "glass-card demo-data-card" }, h("h3", {}, "Recent sample events"));
    var table = h("table", { class: "demo-table" },
      h("thead", {},
        h("tr", {},
          h("th", {}, "Time"),
          h("th", {}, "Action"),
          h("th", {}, "Actor"),
          h("th", {}, "Severity")
        )
      ),
      h("tbody", {},
        demoData.audit.map(function (event) {
          return h("tr", {},
            h("td", {}, event.time),
            h("td", {}, event.action),
            h("td", {}, event.actor),
            h("td", {}, pill(event.severity === "success" ? "low" : event.severity === "warn" ? "high" : "medium", event.severity))
          );
        })
      )
    );
    card.appendChild(table);
    card.appendChild(
      h("div", { class: "demo-callout" }, "The public demo exposes sample audit history only. Customer production audit records belong in customer-owned or explicitly managed environments.")
    );
    el.appendChild(card);
  }

  function renderSettings(el) {
    el.innerHTML = "";
    el.appendChild(
      h("div", { class: "page-header" },
        h("div", {},
          h("h2", {}, "Settings and secrets model"),
          h("p", {}, "This view demonstrates what exists in the product without accepting real customer secrets in the public sandbox.")
        )
      )
    );

    var grid = h("div", { class: "demo-settings-grid" });
    grid.appendChild(
      h("div", { class: "glass-card demo-data-card" },
        h("h3", {}, "Team"),
        h("ul", { class: "demo-list" },
          h("li", {}, h("div", {}, h("strong", {}, "Phil Basile"), h("small", {}, "Admin · hosted beta owner")), pill("Admin", "info")),
          h("li", {}, h("div", {}, h("strong", {}, "Design partner"), h("small", {}, "Editor · invite-only beta access")), pill("Beta", "muted"))
        )
      )
    );
    grid.appendChild(
      h("div", { class: "glass-card demo-data-card" },
        h("h3", {}, "Secrets policy"),
        h("p", {}, "The public demo never stores real API credentials. Hosted beta and customer deployments use the real auth and secret management flow."),
        h("div", { class: "demo-action-row" },
          h("a", { class: "btn-glass", href: "mailto:pbasile@basilecom.com?subject=Mermaduckle%20Hosted%20Beta%20Access" }, "Join hosted beta"),
          h("a", { class: "btn-primary", href: "/docs" }, "Deploy your own instance")
        )
      )
    );
    el.appendChild(grid);
  }

  function renderBuilder(el, idx) {
    var workflow = demoData.workflows[idx];
    if (!workflow) { renderWorkflows(el); return; }

    el.innerHTML = "";
    el.style.padding = "0";
    el.style.display = "flex";
    el.style.flexDirection = "column";

    el.appendChild(
      h("div", { class: "page-header", style: { padding: "1rem 2rem", borderBottom: "1px solid rgba(255,255,255,0.06)", margin: "0" } },
        h("div", { style: { display: "flex", alignItems: "center", gap: "1rem" } },
          h("button", { class: "btn-icon", onClick: function () { renderPage("workflows"); } }, "\u2190"),
          h("div", {},
            h("h2", { style: { fontSize: "1.25rem", marginBottom: "0" } }, workflow.name),
            h("p", { style: { fontSize: "12px" } }, workflow.note)
          )
        ),
        h("div", { style: { display: "flex", alignItems: "center", gap: "0.75rem" } },
          pill(workflow.status, workflow.status === "Active" ? "success" : workflow.status === "Review" ? "info" : "muted"),
          h("span", { style: { fontSize: "12px", color: "var(--slate-500)" } }, workflow.runs + " runs")
        )
      )
    );

    var builderLayout = h("div", { style: { display: "flex", flex: "1", overflow: "hidden" } });

    // Node palette (draggable to canvas)
    var palette = h("div", { style: { width: "220px", background: "rgba(0,0,0,0.2)", borderRight: "1px solid rgba(255,255,255,0.06)", padding: "1rem", display: "flex", flexDirection: "column", gap: "0.75rem", overflowY: "auto" } },
      h("h3", { style: { fontSize: "11px", textTransform: "uppercase", letterSpacing: "0.05em", color: "var(--slate-500)", marginBottom: "0.25rem" } }, "Node Palette"),
      makePaletteItem("\u26A1 Trigger", "var(--amber-400)", "Start event"),
      makePaletteItem("\uD83E\uDD16 Agent", "var(--primary-400)", "LLM processing"),
      makePaletteItem("\uD83D\uDC1D Swarm", "var(--primary-400)", "Parallel agents"),
      makePaletteItem("\uD83D\uDD00 Condition", "var(--cyan-400)", "Branching gate"),
      makePaletteItem("\uD83D\uDD04 Loop", "var(--orange-400)", "Iterative execution"),
      makePaletteItem("\u23F1\uFE0F Delay", "var(--gray-400)", "Time-based pause"),
      makePaletteItem("\u2705 Approval", "var(--violet-400)", "Human review"),
      makePaletteItem("\uD83C\uDF10 HTTP", "var(--blue-400)", "API call"),
      makePaletteItem("\uD83D\uDD28 Action", "var(--emerald-400)", "Integration point")
    );
    builderLayout.appendChild(palette);

    var canvasContainer = h("div", {
      style: { flex: "1", position: "relative", overflow: "hidden", background: "radial-gradient(circle at center, rgba(255,255,255,0.03) 1px, transparent 1px)", backgroundSize: "24px 24px" },
      id: "demo-canvas-container"
    });

    var canvasControls = h("div", { class: "canvas-controls" },
      h("button", { onClick: function () { if (window.zoomCanvas) window.zoomCanvas(0.1); } }, "+"),
      h("button", { onClick: function () { if (window.zoomCanvas) window.zoomCanvas(-0.1); } }, "\u2212"),
      h("button", { onClick: function () { if (window.resetCanvas) window.resetCanvas(); } }, "\u27F2")
    );
    canvasContainer.appendChild(canvasControls);
    builderLayout.appendChild(canvasContainer);
    el.appendChild(builderLayout);

    // Load workflow-canvas.js and initialize
    requestAnimationFrame(function () {
      var canvasData = { nodes: workflow.nodes || [], edges: workflow.edges || [] };
      if (window.initWorkflowCanvas) {
        window.initWorkflowCanvas("demo-canvas-container", canvasData);
      } else {
        var script = document.createElement("script");
        script.src = "/static/workflow-canvas.js";
        script.onload = function () { window.initWorkflowCanvas("demo-canvas-container", canvasData); };
        document.body.appendChild(script);
      }
    });
  }

  function makePaletteItem(label, color, desc) {
    return h("div", {
      class: "glass-card",
      style: { padding: "0.6rem 0.75rem", borderLeft: "3px solid " + color, cursor: "grab", fontSize: "12px" },
      draggable: "true",
      onDragstart: function (e) { e.dataTransfer.setData("text/plain", label); }
    },
      h("div", { style: { fontWeight: "600", color: "white" } }, label),
      h("div", { style: { fontSize: "10px", color: "var(--slate-500)", marginTop: "2px" } }, desc)
    );
  }

  function renderPage(page) {
    var el = $("#demo-page-content");
    if (!el) return;
    currentPage = page;

    // Highlight the correct sidebar nav item
    var navPage = page.indexOf("builder:") === 0 ? "workflows" : page;
    $$(".nav-item[data-demo-page]").forEach(function (item) {
      item.classList.toggle("active", item.getAttribute("data-demo-page") === navPage);
    });

    if (page.indexOf("builder:") === 0) {
      renderBuilder(el, parseInt(page.split(":")[1], 10));
      return;
    }

    // Reset builder styles when navigating to a normal page
    el.style.padding = "";
    el.style.display = "";
    el.style.flexDirection = "";

    switch (page) {
      case "workflows":
        renderWorkflows(el);
        break;
      case "approvals":
        renderApprovals(el);
        break;
      case "agents":
        renderAgents(el);
        break;
      case "audit":
        renderAudit(el);
        break;
      case "settings":
        renderSettings(el);
        break;
      default:
        renderDashboard(el);
        break;
    }
  }

  window.navigateDemo = renderPage;
  document.addEventListener("DOMContentLoaded", function () {
    renderPage(currentPage);
  });
})();
