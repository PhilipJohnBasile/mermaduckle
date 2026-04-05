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
      { name: "Customer support triage", status: "Active", runs: "1,284", owner: "Support Ops", note: "Routes tickets through sentiment scoring and human review." },
      { name: "PR security review", status: "Active", runs: "642", owner: "Platform", note: "Checks risky file changes before merge." },
      { name: "Policy exception intake", status: "Review", runs: "48", owner: "Security", note: "Requires manager approval before escalation." },
      { name: "Outbound research brief", status: "Draft", runs: "0", owner: "Revenue", note: "Builds account context before SDR outreach." }
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
    demoData.workflows.forEach(function (workflow) {
      list.appendChild(
        h("div", { class: "glass-card demo-surface-card" },
          h("div", { style: { display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "flex-start" } },
            h("div", {},
              h("h3", {}, workflow.name),
              h("p", {}, workflow.note)
            ),
            pill(workflow.status, workflow.status === "Active" ? "success" : workflow.status === "Review" ? "info" : "muted")
          ),
          h("div", { class: "demo-section-note" }, "Owner: " + workflow.owner + " · Weekly runs: " + workflow.runs)
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

  function renderPage(page) {
    var el = $("#demo-page-content");
    if (!el) return;
    currentPage = page;
    $$(".nav-item[data-demo-page]").forEach(function (item) {
      item.classList.toggle("active", item.getAttribute("data-demo-page") === page);
    });

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
