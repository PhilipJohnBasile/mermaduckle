/* ═══════════════════════════════════════════════════════════
   Mermaduckle SPA — Workflow Canvas Editor
   Interactive SVG+HTML drag-and-drop node graph canvas
   ═══════════════════════════════════════════════════════════ */

window.initWorkflowCanvas = function(containerId, workflowData) {
  const container = document.getElementById(containerId);
  if (!container) return;
  container.innerHTML = '';

  let nodes = workflowData.nodes || [];
  let edges = workflowData.edges || [];
  let transform = { x: 0, y: 0, scale: 1 };
  let isPanning = false;
  let startPan = { x: 0, y: 0 };
  let draggingNode = null;
  let drawingEdge = null; // { fromNodeId, startX, startY, currentX, currentY }

  // Canvas layers
  const svgLayer = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svgLayer.style.position = 'absolute';
  svgLayer.style.top = '0';
  svgLayer.style.left = '0';
  svgLayer.style.width = '100%';
  svgLayer.style.height = '100%';
  svgLayer.style.overflow = 'visible';
  svgLayer.style.pointerEvents = 'none';

  const edgesGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
  const tempEdge = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  tempEdge.setAttribute('stroke', '#6366f1');
  tempEdge.setAttribute('stroke-width', '2');
  tempEdge.setAttribute('fill', 'none');
  tempEdge.setAttribute('stroke-dasharray', '5,5');
  tempEdge.style.display = 'none';
  
  edgesGroup.appendChild(tempEdge);
  svgLayer.appendChild(edgesGroup);
  container.appendChild(svgLayer);

  const htmlLayer = document.createElement('div');
  htmlLayer.style.position = 'absolute';
  htmlLayer.style.top = '0';
  htmlLayer.style.left = '0';
  htmlLayer.style.width = '100%';
  htmlLayer.style.height = '100%';
  htmlLayer.style.transformOrigin = '0 0';
  container.appendChild(htmlLayer);

  // Apply transformations
  function updateTransform() {
    htmlLayer.style.transform = `translate(${transform.x}px, ${transform.y}px) scale(${transform.scale})`;
    edgesGroup.setAttribute('transform', `translate(${transform.x}, ${transform.y}) scale(${transform.scale})`);
    container.style.backgroundPosition = `${transform.x}px ${transform.y}px`;
    container.style.backgroundSize = `${24 * transform.scale}px ${24 * transform.scale}px`;
  }

  // Draw edges
  function renderEdges() {
    // Keep tempEdge around
    while (edgesGroup.children.length > 1) {
      edgesGroup.removeChild(edgesGroup.lastChild);
    }
    
    edges.forEach(edge => {
      const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      const startNode = nodes.find(n => n.id === edge.source);
      const endNode = nodes.find(n => n.id === edge.target);
      if (!startNode || !endNode) return;
      
      const p1 = { x: startNode.position.x + 200, y: startNode.position.y + 40 }; // right handle
      const p2 = { x: endNode.position.x, y: endNode.position.y + 40 }; // left handle

      const d = `M ${p1.x} ${p1.y} C ${p1.x + 100} ${p1.y}, ${p2.x - 100} ${p2.y}, ${p2.x} ${p2.y}`;
      p.setAttribute('d', d);
      p.setAttribute('stroke', 'var(--slate-400)');
      p.setAttribute('stroke-width', '2');
      p.setAttribute('fill', 'none');
      
      // Edge delete button (on click)
      p.style.pointerEvents = 'stroke';
      p.style.cursor = 'pointer';
      p.onclick = (e) => {
        edges = edges.filter(ed => ed !== edge);
        render();
      };
      
      edgesGroup.appendChild(p);
    });
  }

  // Draw nodes
  function renderNodes() {
    const existingTooltip = document.getElementById('node-tooltip');
    if (existingTooltip) existingTooltip.remove();
    htmlLayer.innerHTML = '';
    nodes.forEach(node => {
      const el = document.createElement('div');
      el.className = 'glass-card workflow-node';
      el.setAttribute('data-node-id', node.id);
      el.style.position = 'absolute';
      el.style.left = `${node.position.x}px`;
      el.style.top = `${node.position.y}px`;
      el.style.width = '200px';
      el.style.pointerEvents = 'auto';
      el.style.userSelect = 'none';
      
      let borderColor = 'rgba(255,255,255,0.1)';
      let icon = '⚙️';
      if (node.type === 'trigger') { borderColor = 'var(--amber-400)'; icon = '⚡'; }
      else if (node.type === 'agent') { borderColor = 'var(--primary-400)'; icon = '🤖'; }
      else if (node.type === 'condition') { borderColor = 'var(--cyan-400)'; icon = '🔀'; }
      else if (node.type === 'approval') { borderColor = 'var(--violet-400)'; icon = '✅'; }
      else if (node.type === 'action') { borderColor = 'var(--emerald-400)'; icon = '🔨'; }
      
      el.style.borderLeft = `3px solid ${borderColor}`;

      el.innerHTML = `
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem">
          <div style="display:flex; align-items:center; gap:0.5rem">
            <span>${icon}</span>
            <span style="font-weight:600; font-size:12px; color:white">${node.name || node.data?.label || 'Untitled'}</span>
          </div>
          <button class="del-btn" style="background:none; border:none; color:var(--slate-500); cursor:pointer">✕</button>
        </div>
        <div style="font-size:10px; color:var(--slate-500); text-transform:uppercase; letter-spacing:0.05em">${node.type}</div>
        
        <div class="handle output-handle" style="position:absolute; right:-6px; top:50%; transform:translateY(-50%); width:12px; height:12px; background:${borderColor}; border-radius:50%; cursor:crosshair; box-shadow:0 0 0 2px var(--bg)"></div>
        <div class="handle input-handle" style="position:absolute; left:-6px; top:50%; transform:translateY(-50%); width:12px; height:12px; background:var(--slate-400); border-radius:50%; box-shadow:0 0 0 2px var(--bg)"></div>
      `;

      // Inspector Tooltip
      el.onmouseenter = () => {
        if (draggingNode || isPanning) return;
        const old = document.getElementById('node-tooltip');
        if (old) old.remove();
        const tooltip = document.createElement('div');
        tooltip.id = 'node-tooltip';
        tooltip.className = 'node-inspector';
        tooltip.innerHTML = `ID: ${node.id}<br>Status: Active`;
        const r = el.getBoundingClientRect();
        tooltip.style.left = `${r.left}px`;
        tooltip.style.top = `${r.bottom + 10}px`;
        document.body.appendChild(tooltip);
      };
      el.onmouseleave = () => {
        const t = document.getElementById('node-tooltip');
        if(t) t.remove();
      };

      // Drag logic
      el.onmousedown = (e) => {
        if (e.target.classList.contains('handle')) return;
        if (e.target.classList.contains('del-btn')) {
          nodes = nodes.filter(n => n.id !== node.id);
          edges = edges.filter(edge => edge.source !== node.id && edge.target !== node.id);
          render();
          return;
        }
        e.stopPropagation();
        draggingNode = node;
      };

      // Edge drawing logic
      const outHandle = el.querySelector('.output-handle');
      outHandle.onmousedown = (e) => {
        e.stopPropagation();
        drawingEdge = {
          from: node.id,
          startX: node.position.x + 200,
          startY: node.position.y + (el.offsetHeight / 2),
        };
        tempEdge.style.display = 'block';
      };

      const inHandle = el.querySelector('.input-handle');
      inHandle.onmouseup = (e) => {
        if (!drawingEdge || drawingEdge.from === node.id) return;
        edges.push({ source: drawingEdge.from, target: node.id });
      };

      htmlLayer.appendChild(el);
    });
  }

  function render() {
    renderNodes();
    renderEdges();
  }

  // Panning & zooming
  container.onmousedown = (e) => {
    isPanning = true;
    startPan = { x: e.clientX - transform.x, y: e.clientY - transform.y };
  };

  window.onmousemove = (e) => {
    if (isPanning) {
      transform.x = e.clientX - startPan.x;
      transform.y = e.clientY - startPan.y;
      updateTransform();
    }
    
    if (draggingNode) {
      const dx = e.movementX / transform.scale;
      const dy = e.movementY / transform.scale;
      draggingNode.position.x += dx;
      draggingNode.position.y += dy;
      render();
    }
    
    if (drawingEdge) {
      const rect = container.getBoundingClientRect();
      const currentX = (e.clientX - rect.left - transform.x) / transform.scale;
      const currentY = (e.clientY - rect.top - transform.y) / transform.scale;
      
      const p1 = { x: drawingEdge.startX, y: drawingEdge.startY };
      const p2 = { x: currentX, y: currentY };
      const d = `M ${p1.x} ${p1.y} C ${p1.x + 100} ${p1.y}, ${p2.x - 100} ${p2.y}, ${p2.x} ${p2.y}`;
      tempEdge.setAttribute('d', d);
    }
  };

  window.onmouseup = () => {
    isPanning = false;
    draggingNode = null;
    if (drawingEdge) {
      drawingEdge = null;
      tempEdge.style.display = 'none';
      render(); // re-render to catch any new edges
    }
  };

  container.onwheel = (e) => {
    e.preventDefault();
    const zoomSensitivity = 0.001;
    const delta = e.deltaY * zoomSensitivity * -1;
    let newScale = transform.scale * (1 + delta);
    newScale = Math.min(Math.max(0.1, newScale), 2);
    
    // Zoom toward cursor
    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    transform.x = x - (x - transform.x) * (newScale / transform.scale);
    transform.y = y - (y - transform.y) * (newScale / transform.scale);
    transform.scale = newScale;
    updateTransform();
  };

  // Drag and drop from palette
  container.ondragover = (e) => e.preventDefault();
  container.ondrop = (e) => {
    e.preventDefault();
    const data = JSON.parse(e.dataTransfer.getData('application/json'));
    const rect = container.getBoundingClientRect();
    const x = (e.clientX - rect.left - transform.x) / transform.scale;
    const y = (e.clientY - rect.top - transform.y) / transform.scale;
    
    nodes.push({
      id: 'node_' + Date.now(),
      type: data.type,
      name: data.label.replace(/[^A-Za-z ]/g, '').trim(),
      position: { x, y }
    });
    render();
  };

  // Save logic
  const saveBtn = document.getElementById('builder-save');
  if (saveBtn) {
    saveBtn.onclick = async () => {
      saveBtn.textContent = 'Saving...';
      const res = await fetch(`/api/workflows/${workflowData.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ nodes, edges })
      });
      if (res.ok) {
        saveBtn.textContent = 'Saved!';
        setTimeout(() => saveBtn.textContent = 'Save Changes', 2000);
      }
    };
  }

  // Exposed API
  window.setWorkflowData = (newNodes, newEdges) => {
    nodes = newNodes;
    edges = newEdges || [];
    render();
  };

  window.getWorkflowNodes = () => nodes;
  window.getWorkflowEdges = () => edges;

  window.zoomCanvas = (delta) => {
    let newScale = transform.scale + delta;
    newScale = Math.min(Math.max(0.1, newScale), 2);
    transform.scale = newScale;
    updateTransform();
  };

  window.resetCanvas = () => {
    transform = { x: 0, y: 0, scale: 1 };
    updateTransform();
  };

  window.autoLayout = () => {
    if (nodes.length === 0) return;
    const padding = 100;
    const colWidth = 300;
    const rowHeight = 120;
    
    // Simple level-based layout
    const levels = {};
    const trigger = nodes.find(n => n.type === 'trigger') || nodes[0];
    levels[trigger.id] = 0;
    
    // BFS for levels
    const queue = [trigger.id];
    const visited = new Set([trigger.id]);
    while (queue.length > 0) {
      const currentId = queue.shift();
      const currentLevel = levels[currentId];
      edges.filter(e => e.source === currentId).forEach(e => {
        if (!visited.has(e.target)) {
          visited.add(e.target);
          levels[e.target] = currentLevel + 1;
          queue.push(e.target);
        }
      });
    }

    const levelCounts = {};
    nodes.forEach(node => {
      const lvl = levels[node.id] || 0;
      const rowIdx = levelCounts[lvl] || 0;
      node.position = {
        x: padding + lvl * colWidth,
        y: padding + rowIdx * rowHeight
      };
      levelCounts[lvl] = rowIdx + 1;
    });
    render();
  };

  // Initial draw
  updateTransform();
  render();
};
