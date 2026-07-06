/* =========================================================================
   Duckweed Cultivation Toolkit — PWA Client-Side Application
   ========================================================================= */

const App = {
  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------
  async api(url, method = 'GET', body = null) {
    const opts = { method, headers: { 'Content-Type': 'application/json' } };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(url, opts);
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      throw new Error(err.detail || 'Request failed');
    }
    return res.json();
  },

  $(id) { return document.getElementById(id); },

  showAlert(container, msg, type = 'danger') {
    const el = typeof container === 'string' ? App.$(container) : container;
    el.innerHTML = `<div class="alert alert-${type}">⚠ ${msg}</div>`;
  },

  renderPpmTable(ppm, title = 'غلظت‌های حاصل (ppm)') {
    let html = `<div class="result-panel"><div class="result-title">${title}</div>`;
    for (const [k, v] of Object.entries(ppm)) {
      html += `<div class="result-row"><span class="result-key">${k}</span><span class="result-value">${typeof v === 'number' ? v.toFixed(4) : v}</span></div>`;
    }
    return html + '</div>';
  },

  statusBadge(status) {
    const map = {
      optimal: 'optimal',
      below_optimal: 'below',
      above_optimal: 'above',
      exceeds_documented_max: 'exceeds',
    };
    const cls = map[status] || 'below';
    const labels = {
      optimal: 'بهینه',
      below_optimal: 'زیر بهینه',
      above_optimal: 'بالای بهینه',
      exceeds_documented_max: 'فراتر از حداکثر',
    };
    return `<span class="badge badge-${cls}">${labels[status] || status}</span>`;
  },

  // ---------------------------------------------------------------------------
  // Navigation
  // ---------------------------------------------------------------------------
  initNav() {
    const setMenuState = (open) => {
      App.$('sidebar').classList.toggle('open', open);
      App.$('sidebarOverlay').classList.toggle('active', open);
      App.$('menuToggle').classList.toggle('menu-open', open);
    };

    document.querySelectorAll('.nav-item a').forEach(link => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const page = link.dataset.page;

        // Update nav active state
        document.querySelectorAll('.nav-item a').forEach(l => l.classList.remove('active'));
        link.classList.add('active');

        // Show target page
        document.querySelectorAll('.page-section').forEach(s => s.classList.remove('active'));
        App.$(`page-${page}`).classList.add('active');

        // Close mobile menu
        setMenuState(false);

        // Load data for the page
        if (page === 'planner') App.planner.loadContainers();
        if (page === 'catalog') App.catalog.loadAll();
        if (page === 'logbook') App.logbook.loadLog();
      });
    });

    // Mobile menu toggle
    App.$('menuToggle').addEventListener('click', () => {
      const open = !App.$('sidebar').classList.contains('open');
      setMenuState(open);
    });
    App.$('sidebarOverlay').addEventListener('click', () => {
      setMenuState(false);
    });
  },

  // ---------------------------------------------------------------------------
  // Tabs
  // ---------------------------------------------------------------------------
  initTabs() {
    document.querySelectorAll('.tab-btn[data-tab]').forEach(btn => {
      btn.addEventListener('click', () => {
        const tabId = btn.dataset.tab;
        const parent = btn.closest('.page-section') || btn.closest('.card');

        // Deactivate siblings
        btn.parentElement.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');

        // Show matching panel
        const panels = parent.querySelectorAll(':scope > .tab-panel, :scope > div > .tab-panel');
        panels.forEach(p => {
          if (p.id === tabId) p.classList.add('active');
          else if (p.id && p.id.startsWith(tabId.split('-')[0] + '-')) p.classList.remove('active');
        });

        // For catalog, just directly handle
        const allPanels = parent.querySelectorAll('.tab-panel');
        const prefix = tabId.split('-')[0];
        allPanels.forEach(p => {
          if (p.id === tabId) p.classList.add('active');
          else if (p.id && p.id.startsWith(prefix + '-')) p.classList.remove('active');
        });
      });
    });

    // Sub-tabs (dosing fwd/rev, ec fwd/rev, stock fwd/rev)
    document.querySelectorAll('.tab-btn[data-subtab]').forEach(btn => {
      btn.addEventListener('click', () => {
        const subtabId = btn.dataset.subtab;
        const group = btn.dataset.group;

        // Deactivate sibling buttons
        btn.parentElement.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');

        // Show matching panel in same card
        const card = btn.closest('.card');
        card.querySelectorAll('.tab-panel').forEach(p => {
          if (p.id === subtabId) p.classList.add('active');
          else if (p.id && p.id.startsWith(group + '-')) p.classList.remove('active');
        });
      });
    });
  },

  // ---------------------------------------------------------------------------
  // Modal
  // ---------------------------------------------------------------------------
  modal: {
    show(html) {
      App.$('modalContent').innerHTML = html;
      App.$('modalBackdrop').classList.add('active');
    },
    hide() {
      App.$('modalBackdrop').classList.remove('active');
      App.$('modalContent').innerHTML = '';
    },
  },

  // ---------------------------------------------------------------------------
  // Container Stock Planner
  // ---------------------------------------------------------------------------
  planner: {
    async loadContainers() {
      try {
        const data = await App.api('/api/catalog/containers');
        const sel = App.$('plannerContainer');
        sel.innerHTML = '';
        for (const [name, p] of Object.entries(data)) {
          const opt = document.createElement('option');
          opt.value = name;
          opt.textContent = `${name} (${p.width_cm}×${p.length_cm} cm, ${p.surface_area_m2} m²)`;
          sel.appendChild(opt);
        }
        if (Object.keys(data).length === 0) {
          sel.innerHTML = '<option value="">— ابتدا ظرفی در کاتالوگ اضافه کنید —</option>';
        }
      } catch (e) {
        App.showAlert('plannerResult', e.message);
      }
    },

    async calculate(e) {
      e.preventDefault();
      const container = App.$('plannerContainer').value;
      if (!container) return App.showAlert('plannerResult', 'لطفاً یک ظرف انتخاب کنید.');
      try {
        const data = await App.api('/api/container-stock', 'POST', {
          container_name: container,
          dosing_interval_days: parseFloat(App.$('plannerInterval').value),
          coverage_fraction: parseFloat(App.$('plannerCoverage').value),
          include_urea: App.$('plannerUrea').checked,
          include_iron: App.$('plannerIron').checked,
          water_depth_cm: parseFloat(App.$('plannerDepth').value),
        });

        let html = '<div class="result-panel">';
        html += '<div class="result-title">مشخصات ظرف</div>';
        html += `<div class="result-row"><span class="result-key">ظرف</span><span class="result-value">${data.container_name}</span></div>`;
        html += `<div class="result-row"><span class="result-key">سطح</span><span class="result-value">${data.surface_area_m2.toFixed(4)} m²</span></div>`;
        html += `<div class="result-row"><span class="result-key">عمق آب</span><span class="result-value">${data.water_depth_cm} cm</span></div>`;
        html += `<div class="result-row"><span class="result-key">حجم آب</span><span class="result-value">${data.vessel_volume_L.toFixed(3)} L</span></div>`;
        html += '</div>';

        html += '<div class="result-panel">';
        html += '<div class="result-title">فرمول محلول ذخیره</div>';
        html += `<div class="result-row"><span class="result-key">عمر ذخیره</span><span class="result-value">${data.stock_lifespan_days} روز</span></div>`;
        html += `<div class="result-row"><span class="result-key">حجم بطری</span><span class="result-value">${data.stock_volume_L} L</span></div>`;
        html += `<div class="result-row"><span class="result-key">تعداد دوزها</span><span class="result-value">${data.number_of_doses}</span></div>`;
        html += `<div class="result-row"><span class="result-key">Valagro</span><span class="result-value">${data.valagro_g_in_stock.toFixed(3)} g</span></div>`;
        if (data.urea_g_in_stock > 0)
          html += `<div class="result-row"><span class="result-key">اوره</span><span class="result-value">${data.urea_g_in_stock.toFixed(3)} g</span></div>`;
        if (data.iron_g_in_stock > 0)
          html += `<div class="result-row"><span class="result-key">آهن کلاته</span><span class="result-value">${data.iron_g_in_stock.toFixed(3)} g</span></div>`;
        html += '</div>';

        html += '<div class="result-panel">';
        html += `<div class="result-title">برنامه دوزدهی (هر ${data.dosing_cycle_days} روز)</div>`;
        html += `<div class="result-row"><span class="result-key">حجم دوز</span><span class="result-value">${data.dose_volume_mL.toFixed(2)} mL</span></div>`;
        if (data.number_of_injections_per_cycle > 1) {
          html += `<div class="alert alert-warning">⚠ تزریق تقسیمی لازم: ${data.number_of_injections_per_cycle} تزریق، هر ${data.injection_interval_days} روز، هر بار ${data.injection_volume_mL} mL</div>`;
        }
        html += '</div>';

        html += App.renderPpmTable(data.cumulative_ppm, 'غلظت‌های تجمعی در هر دوره');

        if (data.warnings && data.warnings.length > 0) {
          for (const w of data.warnings) {
            html += `<div class="alert alert-warning">⚠ ${w}</div>`;
          }
        }

        App.$('plannerResult').innerHTML = html;
      } catch (e) {
        App.showAlert('plannerResult', e.message);
      }
    },
  },

  // ---------------------------------------------------------------------------
  // Catalog Manager
  // ---------------------------------------------------------------------------
  catalog: {
    async loadAll() {
      await Promise.all([
        App.catalog.loadContainers(),
        App.catalog.loadFertilizers(),
        App.catalog.loadLights(),
      ]);
    },

    async loadContainers() {
      try {
        const data = await App.api('/api/catalog/containers');
        const el = App.$('containersList');
        if (Object.keys(data).length === 0) {
          el.innerHTML = '<div class="empty-state"><div class="empty-icon">📦</div><p>هیچ ظرفی ثبت نشده</p></div>';
          return;
        }
        let html = '';
        for (const [name, p] of Object.entries(data)) {
          html += `<div class="catalog-item">
            <div class="catalog-item-info">
              <div class="catalog-item-name">${name}</div>
              <div class="catalog-item-details">${p.width_cm}×${p.length_cm} cm, H: ${p.height_cm} cm, Area: ${p.surface_area_m2} m²</div>
            </div>
            <div class="catalog-item-actions">
              <button class="btn btn-secondary btn-sm" onclick="App.catalog.editContainer('${name}')">ویرایش</button>
              <button class="btn btn-danger btn-sm" onclick="App.catalog.deleteItem('containers', '${name}')">حذف</button>
            </div>
          </div>`;
        }
        el.innerHTML = html;
      } catch (e) { App.showAlert('containersList', e.message); }
    },

    async loadFertilizers() {
      try {
        const data = await App.api('/api/catalog/fertilizers');
        const el = App.$('fertilizersList');
        if (Object.keys(data).length === 0) {
          el.innerHTML = '<div class="empty-state"><div class="empty-icon">🧴</div><p>هیچ کودی ثبت نشده</p></div>';
          return;
        }
        let html = '';
        for (const [name, p] of Object.entries(data)) {
          html += `<div class="catalog-item">
            <div class="catalog-item-info">
              <div class="catalog-item-name">${name}</div>
              <div class="catalog-item-details">N: ${p.N_total}%, P₂O₅: ${p.P2O5}%, K₂O: ${p.K2O}%, MgO: ${p.MgO}%, Fe: ${p.trace_Fe}%</div>
            </div>
            <div class="catalog-item-actions">
              <button class="btn btn-secondary btn-sm" onclick="App.catalog.editFertilizer('${name}')">ویرایش</button>
              <button class="btn btn-danger btn-sm" onclick="App.catalog.deleteItem('fertilizers', '${name}')">حذف</button>
            </div>
          </div>`;
        }
        el.innerHTML = html;
      } catch (e) { App.showAlert('fertilizersList', e.message); }
    },

    async loadLights() {
      try {
        const data = await App.api('/api/catalog/lights');
        const el = App.$('lightsList');
        if (Object.keys(data).length === 0) {
          el.innerHTML = '<div class="empty-state"><div class="empty-icon">💡</div><p>هیچ منبع نوری ثبت نشده</p></div>';
          return;
        }
        let html = '';
        for (const [name, p] of Object.entries(data)) {
          html += `<div class="catalog-item">
            <div class="catalog-item-info">
              <div class="catalog-item-name">${name}</div>
              <div class="catalog-item-details">${p.wattage_W}W, ${p.lumens} lm, ${p.kelvin}K</div>
            </div>
            <div class="catalog-item-actions">
              <button class="btn btn-secondary btn-sm" onclick="App.catalog.editLight('${name}')">ویرایش</button>
              <button class="btn btn-danger btn-sm" onclick="App.catalog.deleteItem('lights', '${name}')">حذف</button>
            </div>
          </div>`;
        }
        el.innerHTML = html;
      } catch (e) { App.showAlert('lightsList', e.message); }
    },

    showAddContainer() {
      App.modal.show(`
        <div class="modal-title">افزودن ظرف جدید</div>
        <form id="addContainerForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="newContName" required></div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">عرض (cm)</label><input class="form-input ltr-input" type="number" id="newContW" step="0.1" required></div>
            <div class="form-group"><label class="form-label">طول (cm)</label><input class="form-input ltr-input" type="number" id="newContL" step="0.1" required></div>
          </div>
          <div class="form-group"><label class="form-label">ارتفاع (cm)</label><input class="form-input ltr-input" type="number" id="newContH" value="5" step="0.1" required></div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">ذخیره</button>
          </div>
        </form>
      `);
      App.$('addContainerForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api('/api/catalog/containers', 'POST', {
            name: App.$('newContName').value,
            width_cm: parseFloat(App.$('newContW').value),
            length_cm: parseFloat(App.$('newContL').value),
            height_cm: parseFloat(App.$('newContH').value),
          });
          App.modal.hide();
          App.catalog.loadContainers();
        } catch (err) { alert(err.message); }
      };
    },

    async editContainer(name) {
      const data = await App.api('/api/catalog/containers');
      const p = data[name];
      if (!p) return;
      App.modal.show(`
        <div class="modal-title">ویرایش ظرف: ${name}</div>
        <form id="editContainerForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="editContName" value="${p.name}" required></div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">عرض (cm)</label><input class="form-input ltr-input" type="number" id="editContW" value="${p.width_cm}" step="0.1" required></div>
            <div class="form-group"><label class="form-label">طول (cm)</label><input class="form-input ltr-input" type="number" id="editContL" value="${p.length_cm}" step="0.1" required></div>
          </div>
          <div class="form-group"><label class="form-label">ارتفاع (cm)</label><input class="form-input ltr-input" type="number" id="editContH" value="${p.height_cm}" step="0.1" required></div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">به‌روزرسانی</button>
          </div>
        </form>
      `);
      App.$('editContainerForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api(`/api/catalog/containers/${encodeURIComponent(name)}`, 'PUT', {
            name: App.$('editContName').value,
            width_cm: parseFloat(App.$('editContW').value),
            length_cm: parseFloat(App.$('editContL').value),
            height_cm: parseFloat(App.$('editContH').value),
          });
          App.modal.hide();
          App.catalog.loadContainers();
        } catch (err) { alert(err.message); }
      };
    },

    showAddFertilizer() {
      App.modal.show(`
        <div class="modal-title">افزودن کود جدید</div>
        <form id="addFertForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="newFertName" required></div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">N (%)</label><input class="form-input ltr-input" type="number" id="newFertN" step="0.01" required></div>
            <div class="form-group"><label class="form-label">P₂O₅ (%)</label><input class="form-input ltr-input" type="number" id="newFertP" step="0.01" required></div>
          </div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">K₂O (%)</label><input class="form-input ltr-input" type="number" id="newFertK" step="0.01" required></div>
            <div class="form-group"><label class="form-label">MgO (%)</label><input class="form-input ltr-input" type="number" id="newFertMg" step="0.01" required></div>
          </div>
          <div class="form-group"><label class="form-label">Fe (%)</label><input class="form-input ltr-input" type="number" id="newFertFe" step="0.001" value="0" required></div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">ذخیره</button>
          </div>
        </form>
      `);
      App.$('addFertForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api('/api/catalog/fertilizers', 'POST', {
            name: App.$('newFertName').value,
            N_total: parseFloat(App.$('newFertN').value),
            P2O5: parseFloat(App.$('newFertP').value),
            K2O: parseFloat(App.$('newFertK').value),
            MgO: parseFloat(App.$('newFertMg').value),
            trace_Fe: parseFloat(App.$('newFertFe').value),
          });
          App.modal.hide();
          App.catalog.loadFertilizers();
        } catch (err) { alert(err.message); }
      };
    },

    async editFertilizer(name) {
      const data = await App.api('/api/catalog/fertilizers');
      const p = data[name];
      if (!p) return;
      App.modal.show(`
        <div class="modal-title">ویرایش کود: ${name}</div>
        <form id="editFertForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="editFertName" value="${p.name}" required></div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">N (%)</label><input class="form-input ltr-input" type="number" id="editFertN" value="${p.N_total}" step="0.01" required></div>
            <div class="form-group"><label class="form-label">P₂O₅ (%)</label><input class="form-input ltr-input" type="number" id="editFertP" value="${p.P2O5}" step="0.01" required></div>
          </div>
          <div class="form-row">
            <div class="form-group"><label class="form-label">K₂O (%)</label><input class="form-input ltr-input" type="number" id="editFertK" value="${p.K2O}" step="0.01" required></div>
            <div class="form-group"><label class="form-label">MgO (%)</label><input class="form-input ltr-input" type="number" id="editFertMg" value="${p.MgO}" step="0.01" required></div>
          </div>
          <div class="form-group"><label class="form-label">Fe (%)</label><input class="form-input ltr-input" type="number" id="editFertFe" value="${p.trace_Fe}" step="0.001" required></div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">به‌روزرسانی</button>
          </div>
        </form>
      `);
      App.$('editFertForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api(`/api/catalog/fertilizers/${encodeURIComponent(name)}`, 'PUT', {
            name: App.$('editFertName').value,
            N_total: parseFloat(App.$('editFertN').value),
            P2O5: parseFloat(App.$('editFertP').value),
            K2O: parseFloat(App.$('editFertK').value),
            MgO: parseFloat(App.$('editFertMg').value),
            trace_Fe: parseFloat(App.$('editFertFe').value),
          });
          App.modal.hide();
          App.catalog.loadFertilizers();
        } catch (err) { alert(err.message); }
      };
    },

    showAddLight() {
      App.modal.show(`
        <div class="modal-title">افزودن منبع نور جدید</div>
        <form id="addLightForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="newLightName" required></div>
          <div class="form-row-3">
            <div class="form-group"><label class="form-label">وات (W)</label><input class="form-input ltr-input" type="number" id="newLightW" step="0.1" required></div>
            <div class="form-group"><label class="form-label">لومن</label><input class="form-input ltr-input" type="number" id="newLightLm" step="1" required></div>
            <div class="form-group"><label class="form-label">کلوین (K)</label><input class="form-input ltr-input" type="number" id="newLightK" step="100" required></div>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">ذخیره</button>
          </div>
        </form>
      `);
      App.$('addLightForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api('/api/catalog/lights', 'POST', {
            name: App.$('newLightName').value,
            wattage_W: parseFloat(App.$('newLightW').value),
            lumens: parseFloat(App.$('newLightLm').value),
            kelvin: parseFloat(App.$('newLightK').value),
          });
          App.modal.hide();
          App.catalog.loadLights();
        } catch (err) { alert(err.message); }
      };
    },

    async editLight(name) {
      const data = await App.api('/api/catalog/lights');
      const p = data[name];
      if (!p) return;
      App.modal.show(`
        <div class="modal-title">ویرایش منبع نور: ${name}</div>
        <form id="editLightForm">
          <div class="form-group"><label class="form-label">نام</label><input class="form-input" id="editLightName" value="${p.name}" required></div>
          <div class="form-row-3">
            <div class="form-group"><label class="form-label">وات (W)</label><input class="form-input ltr-input" type="number" id="editLightW" value="${p.wattage_W}" step="0.1" required></div>
            <div class="form-group"><label class="form-label">لومن</label><input class="form-input ltr-input" type="number" id="editLightLm" value="${p.lumens}" step="1" required></div>
            <div class="form-group"><label class="form-label">کلوین (K)</label><input class="form-input ltr-input" type="number" id="editLightK" value="${p.kelvin}" step="100" required></div>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">به‌روزرسانی</button>
          </div>
        </form>
      `);
      App.$('editLightForm').onsubmit = async (e) => {
        e.preventDefault();
        try {
          await App.api(`/api/catalog/lights/${encodeURIComponent(name)}`, 'PUT', {
            name: App.$('editLightName').value,
            wattage_W: parseFloat(App.$('editLightW').value),
            lumens: parseFloat(App.$('editLightLm').value),
            kelvin: parseFloat(App.$('editLightK').value),
          });
          App.modal.hide();
          App.catalog.loadLights();
        } catch (err) { alert(err.message); }
      };
    },

    async deleteItem(category, name) {
      if (!confirm(`آیا از حذف "${name}" مطمئن هستید؟`)) return;
      try {
        await App.api(`/api/catalog/${category}/${encodeURIComponent(name)}`, 'DELETE');
        App.catalog.loadAll();
      } catch (err) { alert(err.message); }
    },
  },

  // ---------------------------------------------------------------------------
  // Log Book
  // ---------------------------------------------------------------------------
  logbook: {
    async loadLog() {
      try {
        const data = await App.api('/api/log');
        const log = data.log || [];
        const el = App.$('logTable');

        if (log.length === 0) {
          el.innerHTML = '<div class="empty-state"><div class="empty-icon">📒</div><p>هیچ ورودی ثبت نشده</p></div>';
          return;
        }

        let html = `<table class="data-table">
          <thead><tr>
            <th>روز</th><th>منبع نور</th><th>فاصله</th><th>ساعت</th><th>ظروف</th><th>تصاویر</th><th>عملیات</th>
          </tr></thead><tbody>`;
        for (const entry of log) {
          const dist = entry.light_distance_cm != null ? `${entry.light_distance_cm} cm` : '—';
          let hours = '—';
          if (entry.photoperiod_start != null && entry.photoperiod_end != null) {
            const total = entry.photoperiod_end >= entry.photoperiod_start
              ? entry.photoperiod_end - entry.photoperiod_start
              : (24 - entry.photoperiod_start) + entry.photoperiod_end;
            hours = `${total}h`;
          } else if (entry.photoperiod_hours != null) {
            hours = `${entry.photoperiod_hours}h`;
          }
          const containers = Object.keys(entry.containers || {}).join(', ') || '—';
          const imgs = (entry.images || []).map(img => `${img.filename} (${img.description})`).join(', ') || '—';
          const ops = (entry.operations || []).length;
          html += `<tr>
            <td>${entry.day}</td>
            <td>${entry.light_source || '—'}</td>
            <td>${dist}</td>
            <td>${hours}</td>
            <td>${containers}</td>
            <td>${imgs}</td>
            <td>${ops}</td>
          </tr>`;
        }
        html += '</tbody></table>';
        el.innerHTML = html;
      } catch (e) { App.showAlert('logTable', e.message); }
    },

    async showAddEntry() {
      const data = await App.api('/api/log');
      const lightTypes = Object.keys(data.light_types || {});
      const containerTypes = Object.keys(data.container_types || {});
      const log = data.log || [];
      const nextDay = log.length > 0 ? Math.max(...log.map(e => e.day)) + 1 : 1;

      let lightOpts = lightTypes.map(n => `<option value="${n}">${n}</option>`).join('');
      if (lightOpts === '') lightOpts = '<option value="">— ابتدا نوعی در کاتالوگ اضافه کنید —</option>';

      let contTypeOpts = containerTypes.map(n => `<option value="${n}">${n}</option>`).join('');
      if (contTypeOpts === '') contTypeOpts = '<option value="">— ابتدا ظرفی در کاتالوگ اضافه کنید —</option>';

      App.modal.show(`
        <div class="modal-title">ثبت روز جدید</div>
        <form id="addLogForm">
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">شماره روز</label>
              <input class="form-input ltr-input" type="number" id="logDay" value="${nextDay}" required>
            </div>
            <div class="form-group">
              <label class="form-label">منبع نور</label>
              <select class="form-select" id="logLight">${lightOpts}</select>
            </div>
          </div>
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">فاصله نور (cm)</label>
              <input class="form-input ltr-input" type="number" id="logDist" step="0.1">
            </div>
            <div class="form-group">
              <label class="form-label">شروع نوردهی (ساعت)</label>
              <input class="form-input ltr-input" type="number" id="logHoursStart" step="1" min="0" max="23" placeholder="مثلاً 8">
            </div>
            <div class="form-group">
              <label class="form-label">پایان نوردهی (ساعت)</label>
              <input class="form-input ltr-input" type="number" id="logHoursEnd" step="1" min="0" max="23" placeholder="مثلاً 20">
            </div>
          </div>
          <div class="form-group">
            <label class="form-label" style="display:flex; justify-content:space-between; align-items:center;">
              <span>ظروف کشت</span>
              <button type="button" class="btn btn-secondary btn-sm" onclick="App.logbook.addContainerField()">+ افزودن ظرف</button>
            </label>
            <div id="logContainersContainer" style="display:flex; flex-direction:column; gap:var(--space-sm); margin-top:var(--space-xs);"></div>
          </div>
          <div class="form-group">
            <label class="form-label">عملیات (هر خط یکی)</label>
            <textarea class="form-textarea" id="logOps" placeholder="هر عملیات در یک خط ..."></textarea>
          </div>
          <div class="form-group">
            <label class="form-label">مشاهدات (هر خط یکی)</label>
            <textarea class="form-textarea" id="logObs" placeholder="هر مشاهده در یک خط ..."></textarea>
          </div>
          <div class="form-group">
            <label class="form-label">بحث‌ها (هر خط یکی)</label>
            <textarea class="form-textarea" id="logDisc" placeholder="هر بحث در یک خط ..."></textarea>
          </div>
          <div class="form-group">
            <label class="form-label" style="display:flex; justify-content:space-between; align-items:center;">
              <span>تصاویر روز</span>
              <button type="button" class="btn btn-secondary btn-sm" onclick="App.logbook.addImageField()">+ افزودن تصویر</button>
            </label>
            <div id="logImagesContainer" style="display:flex; flex-direction:column; gap:var(--space-xs); margin-top:var(--space-xs);"></div>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" onclick="App.modal.hide()">انصراف</button>
            <button type="submit" class="btn btn-primary">ذخیره</button>
          </div>
        </form>
      `);

      App._contTypeOpts = contTypeOpts;

      App.$('addLogForm').onsubmit = async (e) => {
        e.preventDefault();
        const lines = (id) => App.$(id).value.split('\n').map(l => l.trim()).filter(l => l);

        // Collect containers from dynamic fields
        const containers = {};
        App.$('logContainersContainer').querySelectorAll('.container-entry-row').forEach(row => {
          const cid = row.querySelector('.cont-cid').value.trim();
          if (!cid) return;
          containers[cid] = {
            type: row.querySelector('.cont-type').value,
            water_depth_cm: parseFloat(row.querySelector('.cont-depth').value) || 1.5,
            coverage_percent: parseFloat(row.querySelector('.cont-cov').value) || 80,
            tds_ppm: row.querySelector('.cont-tds').value ? parseInt(row.querySelector('.cont-tds').value) : null,
            biomass_status: row.querySelector('.cont-status').value || 'healthy',
          };
        });

        const images = Array.from(App.$('logImagesContainer').children).map(row => {
          const filename = row.querySelector('.img-filename')?.value?.trim() || '';
          const description = row.querySelector('.img-desc')?.value?.trim() || '';
          return { filename, description };
        }).filter(img => img.filename);

        try {
          await App.api('/api/log', 'POST', {
            day: parseInt(App.$('logDay').value),
            light_source: App.$('logLight').value,
            light_distance_cm: App.$('logDist').value ? parseFloat(App.$('logDist').value) : null,
            photoperiod_start: App.$('logHoursStart').value ? parseFloat(App.$('logHoursStart').value) : null,
            photoperiod_end: App.$('logHoursEnd').value ? parseFloat(App.$('logHoursEnd').value) : null,
            containers: containers,
            operations: lines('logOps'),
            observations: lines('logObs'),
            discussions: lines('logDisc'),
            images: images,
          });
          App.modal.hide();
          App.logbook.loadLog();
        } catch (err) { alert(err.message); }
      };
    },

    addContainerField() {
      const container = App.$('logContainersContainer');
      const opts = App._contTypeOpts || '';
      const div = document.createElement('div');
      div.className = 'container-entry-row';
      div.style.cssText = 'display:grid; grid-template-columns:1fr 1fr; gap:var(--space-xs); padding:var(--space-sm); border:1px solid var(--border-subtle); border-radius:var(--radius-sm); position:relative;';
      div.innerHTML = `
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">نام ظرف</label>
          <input class="form-input cont-cid" type="text" placeholder="مثلاً A1" required>
        </div>
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">نوع</label>
          <select class="form-select cont-type">${opts}</select>
        </div>
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">عمق آب (cm)</label>
          <input class="form-input ltr-input cont-depth" type="number" value="1.5" step="0.1">
        </div>
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">پوشش (%)</label>
          <input class="form-input ltr-input cont-cov" type="number" value="80" step="5" min="0" max="100">
        </div>
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">TDS (ppm)</label>
          <input class="form-input ltr-input cont-tds" type="number" step="1">
        </div>
        <div class="form-group" style="margin:0;">
          <label class="form-label" style="font-size:var(--font-size-xs);">وضعیت</label>
          <input class="form-input cont-status" type="text" value="healthy">
        </div>
        <button type="button" class="btn btn-danger btn-sm" onclick="this.closest('.container-entry-row').remove()" style="position:absolute; top:4px; left:4px; padding:2px 6px; font-size:10px;">✕</button>
      `;
      container.appendChild(div);
    },

    addImageField() {
      const container = App.$('logImagesContainer');
      const rowId = 'imgRow_' + Date.now();
      const div = document.createElement('div');
      div.id = rowId;
      div.className = 'form-row';
      div.style.marginBottom = 'var(--space-xs)';
      div.style.gap = 'var(--space-xs)';
      div.style.alignItems = 'center';

      if (window.AndroidInterface && typeof window.AndroidInterface.pickImageForLog === 'function') {
        div.innerHTML = `
          <input class="form-input ltr-input img-filename" type="text" placeholder="نام فایل" style="flex:2;" readonly>
          <input class="form-input img-desc" type="text" placeholder="توضیح تصویر" style="flex:3;">
          <button type="button" class="btn btn-secondary btn-sm" onclick="App.logbook.pickImage('${rowId}')" style="padding:var(--space-xs) var(--space-sm); margin:0;">انتخاب</button>
          <button type="button" class="btn btn-danger btn-sm" onclick="this.closest('.form-row').remove()" style="padding:var(--space-xs) var(--space-sm); margin:0;">✕</button>
        `;
      } else {
        div.innerHTML = `
          <input class="form-input ltr-input img-filename" type="text" placeholder="نام فایل (مثلا img1.jpg)" style="flex:2;" required>
          <input class="form-input img-desc" type="text" placeholder="توضیح تصویر" style="flex:3;" required>
          <button type="button" class="btn btn-danger btn-sm" onclick="this.closest('.form-row').remove()" style="padding:var(--space-xs) var(--space-sm); margin:0;">✕</button>
        `;
      }
      container.appendChild(div);
    },

    pickImage(rowId) {
      window._imagePickCallback = function(filename) {
        const row = document.getElementById(rowId);
        if (row) {
          const filenameInput = row.querySelector('.img-filename');
          if (filenameInput) filenameInput.value = filename;
          const descInput = row.querySelector('.img-desc');
          if (descInput && !descInput.value) descInput.value = filename.replace(/\.[^.]+$/, '');
        }
        window._imagePickCallback = null;
      };
      window.AndroidInterface.pickImageForLog(rowId);
    },

    async exportMarkdown() {
      try {
        const data = await App.api('/api/log');
        const log = data.log || [];
        if (log.length === 0) {
          alert('هیچ ورودی برای خروجی وجود ندارد.');
          return;
        }

        let md = '# Project BioMesh: Cultivation Log Book\n\n';
        md += '> This log records environmental parameters, nutritional dosages, and physiological responses\n';
        md += '> of Lemna/Wolffia colonies in home cultivation trials.\n\n';
        md += '## Daily Cultivation Logs\n\n';

        for (const entry of log) {
          md += `### Day ${entry.day}\n\n`;
          const light = entry.light_source || 'Unspecified';
          const dist = entry.light_distance_cm != null ? `${entry.light_distance_cm} cm` : 'Not logged';
          let hours = 'Not logged';
          if (entry.photoperiod_start != null && entry.photoperiod_end != null) {
            const total = entry.photoperiod_end >= entry.photoperiod_start
              ? entry.photoperiod_end - entry.photoperiod_start
              : (24 - entry.photoperiod_start) + entry.photoperiod_end;
            hours = `${entry.photoperiod_start}:00 to ${entry.photoperiod_end}:00 (${total} hours)`;
          } else if (entry.photoperiod_hours != null) {
            hours = `${entry.photoperiod_hours} hours`;
          }
          md += `* **Light Source:** ${light} | **Distance:** ${dist} | **Photoperiod:** ${hours}\n\n`;

          const containers = entry.containers || {};
          if (Object.keys(containers).length > 0) {
            md += '| Container | Type | Water Depth (cm) | Coverage (%) | TDS (ppm) | Status |\n';
            md += '|:---:|:---:|:---:|:---:|:---:|:---:|\n';
            for (const [cid, c] of Object.entries(containers)) {
              md += `| **${cid}** | ${c.type || '-'} | ${c.water_depth_cm || 1.5} | ${c.coverage_percent != null ? c.coverage_percent + '%' : '-'} | ${c.tds_ppm || '-'} | ${c.biomass_status || 'healthy'} |\n`;
            }
            md += '\n';
          }

          if ((entry.operations || []).length > 0) {
            md += '**Operations:**\n';
            entry.operations.forEach(op => md += `- ${op}\n`);
            md += '\n';
          }
          if ((entry.observations || []).length > 0) {
            md += '**Observations:**\n';
            entry.observations.forEach(o => md += `- ${o}\n`);
            md += '\n';
          }
          if ((entry.discussions || []).length > 0) {
            md += '**Discussions:**\n';
            entry.discussions.forEach(d => md += `- ${d}\n`);
            md += '\n';
          }
          if ((entry.images || []).length > 0) {
            md += '**Images:**\n';
            entry.images.forEach(img => md += `![${img.description || ''}](images/${img.filename})\n`);
            md += '\n';
          }
          md += '---\n\n';
        }

        if (window.AndroidInterface && typeof window.AndroidInterface.exportMarkdown === 'function') {
          window.AndroidInterface.exportMarkdown(md);
        } else if (window.AndroidInterface && typeof window.AndroidInterface.exportDatabase === 'function') {
          window.AndroidInterface.exportDatabase(md);
        } else {
          const blob = new Blob([md], { type: 'text/markdown' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = 'cultivation_log.md';
          document.body.appendChild(a);
          a.click();
          document.body.removeChild(a);
          URL.revokeObjectURL(url);
        }
      } catch (e) { alert(e.message); }
    },

    async exportJSON() {
      try {
        const data = await App.api('/api/db/export');
        const str = JSON.stringify(data, null, 2);
        
        if (window.AndroidInterface && typeof window.AndroidInterface.exportDatabase === 'function') {
          window.AndroidInterface.exportDatabase(str);
        } else {
          // Standard browser download
          const blob = new Blob([str], { type: 'application/json' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = 'duckweed_database.json';
          document.body.appendChild(a);
          a.click();
          document.body.removeChild(a);
          URL.revokeObjectURL(url);
        }
      } catch (e) { alert(e.message); }
    },

    triggerImportJSON() {
      if (window.AndroidInterface && typeof window.AndroidInterface.importDatabase === 'function') {
        window.AndroidInterface.importDatabase();
      } else {
        App.$('importJsonInput').click();
      }
    },

    async handleImportJSON(event) {
      const file = event.target.files[0];
      if (!file) return;
      
      const reader = new FileReader();
      reader.onload = async (e) => {
        try {
          const json = JSON.parse(e.target.result);
          
          if (!json.container_types || !json.log) {
            throw new Error('فایل انتخاب شده ساختار پایگاه داده معتبر Duckweed را ندارد.');
          }
          
          await App.api('/api/db/import', 'POST', json);
          alert('پایگاه داده با موفقیت بازیابی شد.');
          window.location.reload();
        } catch (err) {
          alert('خطا در بارگذاری فایل: ' + err.message);
        }
      };
      reader.readAsText(file);
      event.target.value = '';
    },

    triggerImportImages() {
      if (window.AndroidInterface && typeof window.AndroidInterface.importImages === 'function') {
        window.AndroidInterface.importImages();
      } else {
        App.$('importImagesInput').click();
      }
    },

    async exportImages() {
      try {
        const data = await App.api('/api/images');
        const images = data.images || [];
        if (images.length === 0) {
          alert('هیچ تصویری برای خروجی وجود ندارد.');
          return;
        }

        if (window.AndroidInterface && typeof window.AndroidInterface.exportImages === 'function') {
          window.AndroidInterface.exportImages(JSON.stringify(images));
        } else {
          // Browser fallback: download each image
          for (const img of images) {
            const a = document.createElement('a');
            a.href = `/api/images/file/${encodeURIComponent(img.filename)}`;
            a.download = img.filename;
            a.click();
          }
        }
      } catch (e) { alert(e.message); }
    },

    async handleImportImages(event) {
      const files = event.target.files;
      if (!files || files.length === 0) return;

      const formData = new FormData();
      for (const file of files) {
        formData.append('images', file, file.name);
      }

      try {
        const res = await fetch('/api/images/import', {
          method: 'POST',
          body: formData,
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({ detail: 'Upload failed' }));
          throw new Error(err.detail || 'Upload failed');
        }
        const result = await res.json();
        let msg = `${result.saved_count} تصویر ذخیره شد.`;
        if (result.correlated_count > 0) {
          msg += `\n${result.correlated_count} تصویر با ورودی‌های دفتر ثبت همبسته شد.`;
        }
        alert(msg);
        App.logbook.loadLog();
      } catch (err) {
        alert('خطا در بارگذاری تصاویر: ' + err.message);
      }
      event.target.value = '';
    },
  },

  // ---------------------------------------------------------------------------
  // Calculators
  // ---------------------------------------------------------------------------
  calculators: {
    async dosingForward(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/dosing/forward', 'POST', {
          dose_g_per_L: parseFloat(App.$('dosingFwdDose').value),
          water_volume_L: parseFloat(App.$('dosingFwdVolume').value),
          source: App.$('dosingFwdSource').value,
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه</div>';
        html += `<div class="result-row"><span class="result-key">منبع</span><span class="result-value">${data.source_name}</span></div>`;
        html += `<div class="result-row"><span class="result-key">وزن کل</span><span class="result-value">${data.total_grams.toFixed(3)} g</span></div>`;
        html += '</div>';
        html += App.renderPpmTable(data.ppm);
        App.$('dosingFwdResult').innerHTML = html;
      } catch (err) { App.showAlert('dosingFwdResult', err.message); }
    },

    async dosingReverse(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/dosing/reverse', 'POST', {
          target_ppm: parseFloat(App.$('dosingRevTarget').value),
          nutrient: App.$('dosingRevNutrient').value,
          water_volume_L: parseFloat(App.$('dosingRevVolume').value),
          source: App.$('dosingRevSource').value,
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه</div>';
        html += `<div class="result-row"><span class="result-key">منبع</span><span class="result-value">${data.source_name}</span></div>`;
        html += `<div class="result-row"><span class="result-key">دوز مورد نیاز</span><span class="result-value">${data.dose_g_per_L.toFixed(4)} g/L</span></div>`;
        html += `<div class="result-row"><span class="result-key">وزن کل</span><span class="result-value">${data.total_grams.toFixed(3)} g</span></div>`;
        html += '</div>';
        html += App.renderPpmTable(data.ppm);
        App.$('dosingRevResult').innerHTML = html;
      } catch (err) { App.showAlert('dosingRevResult', err.message); }
    },

    async ecForward(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/ec/forward', 'POST', {
          dose_g_per_L: parseFloat(App.$('ecFwdDose').value),
          scale: App.$('ecFwdScale').value,
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه EC</div>';
        html += `<div class="result-row"><span class="result-key">TDS تقریبی</span><span class="result-value">${data.total_dissolved_solids_ppm.toFixed(1)} ppm</span></div>`;
        html += `<div class="result-row"><span class="result-key">مقیاس</span><span class="result-value">1:${data.scale}</span></div>`;
        html += `<div class="result-row"><span class="result-key">EC تخمینی</span><span class="result-value">${data.estimated_EC_mS_cm} mS/cm</span></div>`;
        html += '</div>';
        App.$('ecFwdResult').innerHTML = html;
      } catch (err) { App.showAlert('ecFwdResult', err.message); }
    },

    async ecReverse(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/ec/reverse', 'POST', {
          target_ec: parseFloat(App.$('ecRevTarget').value),
          scale: App.$('ecRevScale').value,
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه</div>';
        html += `<div class="result-row"><span class="result-key">دوز مورد نیاز</span><span class="result-value">${data.dose_g_per_L} g/L</span></div>`;
        html += `<div class="result-row"><span class="result-key">TDS تقریبی</span><span class="result-value">${data.total_dissolved_solids_ppm.toFixed(1)} ppm</span></div>`;
        html += `<div class="result-row"><span class="result-key">EC</span><span class="result-value">${data.estimated_EC_mS_cm} mS/cm</span></div>`;
        html += '</div>';
        App.$('ecRevResult').innerHTML = html;
      } catch (err) { App.showAlert('ecRevResult', err.message); }
    },

    async stockForward(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/stock/forward', 'POST', {
          final_dose_g_per_L: parseFloat(App.$('stockFwdDose').value),
          dilution_ratio: parseFloat(App.$('stockFwdRatio').value),
          stock_volume_L: parseFloat(App.$('stockFwdVol').value),
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه محلول ذخیره</div>';
        html += `<div class="result-row"><span class="result-key">حجم بطری</span><span class="result-value">${data.stock_volume_L} L</span></div>`;
        html += `<div class="result-row"><span class="result-key">کود مورد نیاز</span><span class="result-value">${data.stock_grams.toFixed(3)} g</span></div>`;
        html += `<div class="result-row"><span class="result-key">غلظت ذخیره</span><span class="result-value">${data.stock_dose_g_per_L.toFixed(3)} g/L</span></div>`;
        html += `<div class="result-row"><span class="result-key">نسبت رقیق‌سازی</span><span class="result-value">1:${data.dilution_ratio}</span></div>`;
        html += `<div class="result-row"><span class="result-key">دوز نهایی</span><span class="result-value">${data.final_dose_g_per_L.toFixed(4)} g/L</span></div>`;
        html += '</div>';
        html += App.renderPpmTable(data.final_ppm, 'ppm نهایی تحویلی به گیاه');
        App.$('stockFwdResult').innerHTML = html;
      } catch (err) { App.showAlert('stockFwdResult', err.message); }
    },

    async stockReverse(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/stock/reverse', 'POST', {
          stock_grams: parseFloat(App.$('stockRevGrams').value),
          stock_volume_L: parseFloat(App.$('stockRevVol').value),
          dilution_ratio: parseFloat(App.$('stockRevRatio').value),
        });
        let html = '<div class="result-panel"><div class="result-title">نتیجه</div>';
        html += `<div class="result-row"><span class="result-key">غلظت ذخیره</span><span class="result-value">${data.stock_dose_g_per_L.toFixed(3)} g/L</span></div>`;
        html += `<div class="result-row"><span class="result-key">دوز نهایی</span><span class="result-value">${data.final_dose_g_per_L.toFixed(4)} g/L</span></div>`;
        html += '</div>';
        html += App.renderPpmTable(data.final_ppm, 'ppm نهایی تحویلی به گیاه');
        App.$('stockRevResult').innerHTML = html;
      } catch (err) { App.showAlert('stockRevResult', err.message); }
    },
  },

  // ---------------------------------------------------------------------------
  // Vessel Simulator
  // ---------------------------------------------------------------------------
  simulator: {
    async run(e) {
      e.preventDefault();
      try {
        const data = await App.api('/api/simulator', 'POST', {
          volume_L: parseFloat(App.$('simVolume').value),
          width_cm: parseFloat(App.$('simWidth').value),
          length_cm: parseFloat(App.$('simLength').value),
          valagro_g_per_week: parseFloat(App.$('simValagro').value),
          urea_g_per_week: parseFloat(App.$('simUrea').value),
          iron_g_per_week: parseFloat(App.$('simIron').value),
          weeks: parseInt(App.$('simWeeks').value),
          exchange_fraction: parseFloat(App.$('simExchange').value),
        });

        const nutrientLabels = {
          NO3_N: 'نیترات (N)',
          NH4_N: 'آمونیوم (N)',
          P: 'فسفر (P)',
          K: 'پتاسیم (K)',
          Mg: 'منیزیم (Mg)',
          Fe: 'آهن (Fe)',
        };

        let html = `<div class="result-panel">
          <div class="result-title">نتایج شبیه‌سازی</div>
          <div class="result-row"><span class="result-key">حجم ظرف</span><span class="result-value">${data.volume_L} L</span></div>
          <div class="result-row"><span class="result-key">سطح</span><span class="result-value">${data.surface_area_m2.toFixed(3)} m²</span></div>
        </div>`;

        // Build results table
        if (data.weeks && data.weeks.length > 0) {
          const nutrients = Object.keys(data.weeks[0].concentrations).filter(k =>
            ['NO3_N', 'NH4_N', 'P', 'K', 'Mg', 'Fe'].includes(k)
          );

          html += '<div style="overflow-x:auto;"><table class="data-table"><thead><tr><th>هفته</th>';
          for (const n of nutrients) html += `<th>${nutrientLabels[n] || n}</th>`;
          html += '<th>وضعیت</th></tr></thead><tbody>';

          for (const week of data.weeks) {
            html += `<tr><td>${week.week}</td>`;
            for (const n of nutrients) {
              const val = week.concentrations[n];
              const formatted = val != null ? (val >= 10 ? val.toFixed(1) : val.toFixed(2)) : '—';
              html += `<td style="font-family:var(--font-mono)">${formatted}</td>`;
            }
            // Status flags
            const flags = Object.entries(week.statuses)
              .filter(([_, s]) => s !== 'optimal')
              .map(([k, s]) => `${k}: ${App.statusBadge(s)}`);
            html += `<td>${flags.length > 0 ? flags.join(' ') : '<span class="badge badge-optimal">OK</span>'}</td>`;
            html += '</tr>';
          }
          html += '</tbody></table></div>';
        }

        App.$('simulatorResult').innerHTML = html;
      } catch (err) { App.showAlert('simulatorResult', err.message); }
    },
  },

  // ---------------------------------------------------------------------------
  // Init
  // ---------------------------------------------------------------------------
  init() {
    App.initNav();
    App.initTabs();

    // Bind forms
    App.$('plannerForm').addEventListener('submit', App.planner.calculate);
    App.$('dosingFwdForm').addEventListener('submit', App.calculators.dosingForward);
    App.$('dosingRevForm').addEventListener('submit', App.calculators.dosingReverse);
    App.$('ecFwdForm').addEventListener('submit', App.calculators.ecForward);
    App.$('ecRevForm').addEventListener('submit', App.calculators.ecReverse);
    App.$('stockFwdForm').addEventListener('submit', App.calculators.stockForward);
    App.$('stockRevForm').addEventListener('submit', App.calculators.stockReverse);
    App.$('simulatorForm').addEventListener('submit', App.simulator.run);

    // Close modal on backdrop click
    App.$('modalBackdrop').addEventListener('click', (e) => {
      if (e.target === App.$('modalBackdrop')) App.modal.hide();
    });

    // Load initial data for the default page
    App.planner.loadContainers();

    // Register service worker
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.register('/web/sw.js').catch(() => {});
    }
  },
};

document.addEventListener('DOMContentLoaded', App.init);
