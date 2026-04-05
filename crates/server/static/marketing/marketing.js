(function () {
  if (!document.body.classList.contains("marketing-page")) {
    return;
  }

  var prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  initMenuToggle();
  initRevealObserver(prefersReducedMotion);
  initCopyButtons();
  initTrackedLinks(".marketing-nav a[href^='#']");
  initTrackedLinks(".marketing-doc-links a[href^='#']");
  initConsoleTabs(prefersReducedMotion);
  initScrollProgress();
  initSpotlights(prefersReducedMotion);

  function initMenuToggle() {
    var toggle = document.querySelector("[data-marketing-menu-toggle]");
    if (!toggle) {
      return;
    }

    var closeMenu = function () {
      document.body.classList.remove("marketing-nav-open");
      toggle.setAttribute("aria-expanded", "false");
    };

    toggle.addEventListener("click", function () {
      var willOpen = !document.body.classList.contains("marketing-nav-open");
      document.body.classList.toggle("marketing-nav-open", willOpen);
      toggle.setAttribute("aria-expanded", willOpen ? "true" : "false");
    });

    window.addEventListener("resize", function () {
      if (window.innerWidth > 880) {
        closeMenu();
      }
    });

    Array.prototype.forEach.call(document.querySelectorAll(".marketing-nav a"), function (link) {
      link.addEventListener("click", closeMenu);
    });
  }

  function initRevealObserver(reducedMotion) {
    var items = document.querySelectorAll("[data-reveal]");
    if (!items.length) {
      return;
    }

    if (reducedMotion || !("IntersectionObserver" in window)) {
      Array.prototype.forEach.call(items, function (item) {
        item.classList.add("is-visible");
      });
      return;
    }

    var observer = new IntersectionObserver(
      function (entries) {
        Array.prototype.forEach.call(entries, function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.18, rootMargin: "0px 0px -8% 0px" }
    );

    Array.prototype.forEach.call(items, function (item) {
      observer.observe(item);
    });
  }

  function initCopyButtons() {
    if (!navigator.clipboard) {
      return;
    }

    Array.prototype.forEach.call(document.querySelectorAll("pre[data-copyable]"), function (block) {
      var button = document.createElement("button");
      var resetTimer = 0;
      var sourceText = (block.querySelector("code") || block).innerText.trim();

      button.className = "marketing-copy-button";
      button.type = "button";
      button.textContent = "Copy";

      button.addEventListener("click", function () {
        navigator.clipboard.writeText(sourceText).then(
          function () {
            button.textContent = "Copied";
            window.clearTimeout(resetTimer);
            resetTimer = window.setTimeout(function () {
              button.textContent = "Copy";
            }, 1600);
          },
          function () {
            button.textContent = "Failed";
            window.clearTimeout(resetTimer);
            resetTimer = window.setTimeout(function () {
              button.textContent = "Copy";
            }, 1600);
          }
        );
      });

      block.classList.add("marketing-copy-ready");
      block.appendChild(button);
    });
  }

  function initTrackedLinks(selector) {
    var links = Array.prototype.slice.call(document.querySelectorAll(selector));
    if (!links.length || !("IntersectionObserver" in window)) {
      return;
    }

    var sections = links
      .map(function (link) {
        var target = link.getAttribute("href");
        return target && target.charAt(0) === "#" ? document.querySelector(target) : null;
      })
      .filter(Boolean);

    if (!sections.length) {
      return;
    }

    var setActive = function (id) {
      Array.prototype.forEach.call(links, function (link) {
        var active = link.getAttribute("href") === "#" + id;
        link.classList.toggle("is-active", active);
        if (active) {
          link.setAttribute("aria-current", "true");
        } else {
          link.removeAttribute("aria-current");
        }
      });
    };

    var observer = new IntersectionObserver(
      function (entries) {
        var visibleEntry = entries
          .filter(function (entry) {
            return entry.isIntersecting;
          })
          .sort(function (left, right) {
            return right.intersectionRatio - left.intersectionRatio;
          })[0];

        if (visibleEntry && visibleEntry.target.id) {
          setActive(visibleEntry.target.id);
        }
      },
      { threshold: [0.18, 0.4, 0.72], rootMargin: "-30% 0px -45% 0px" }
    );

    Array.prototype.forEach.call(sections, function (section) {
      observer.observe(section);
    });

    if (window.location.hash) {
      setActive(window.location.hash.slice(1));
    } else if (sections[0].id) {
      setActive(sections[0].id);
    }
  }

  function initConsoleTabs(reducedMotion) {
    var tabs = Array.prototype.slice.call(document.querySelectorAll("[data-console-tab]"));
    var panels = Array.prototype.slice.call(document.querySelectorAll("[data-console-panel]"));
    var rotationTimer = 0;

    if (!tabs.length || !panels.length) {
      return;
    }

    var setActive = function (name) {
      Array.prototype.forEach.call(tabs, function (tab) {
        var isActive = tab.getAttribute("data-console-tab") === name;
        tab.classList.toggle("is-active", isActive);
        tab.setAttribute("aria-selected", isActive ? "true" : "false");
      });

      Array.prototype.forEach.call(panels, function (panel) {
        var isActive = panel.getAttribute("data-console-panel") === name;
        panel.classList.toggle("is-active", isActive);
        panel.hidden = !isActive;
      });
    };

    var scheduleRotation = function () {
      if (reducedMotion) {
        return;
      }

      window.clearInterval(rotationTimer);
      rotationTimer = window.setInterval(function () {
        if (document.hidden) {
          return;
        }

        var currentIndex = tabs.findIndex(function (tab) {
          return tab.classList.contains("is-active");
        });
        var nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % tabs.length;
        setActive(tabs[nextIndex].getAttribute("data-console-tab"));
      }, 4200);
    };

    Array.prototype.forEach.call(tabs, function (tab) {
      tab.addEventListener("click", function () {
        setActive(tab.getAttribute("data-console-tab"));
        scheduleRotation();
      });
    });

    setActive(tabs[0].getAttribute("data-console-tab"));

    if (!reducedMotion) {
      scheduleRotation();
    }
  }

  function initScrollProgress() {
    var bar = document.querySelector("[data-scroll-progress]");
    if (!bar) {
      return;
    }

    var update = function () {
      var scrollTop = window.scrollY || window.pageYOffset;
      var docHeight = document.documentElement.scrollHeight - window.innerHeight;
      var progress = docHeight > 0 ? Math.min(scrollTop / docHeight, 1) : 0;
      bar.style.setProperty("--marketing-progress", String(progress * 100) + "%");
    };

    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
  }

  function initSpotlights(reducedMotion) {
    if (reducedMotion) {
      return;
    }

    Array.prototype.forEach.call(document.querySelectorAll("[data-spotlight]"), function (panel) {
      var updateSpotlight = function (event) {
        var rect = panel.getBoundingClientRect();
        var x = ((event.clientX - rect.left) / rect.width) * 100;
        var y = ((event.clientY - rect.top) / rect.height) * 100;
        panel.style.setProperty("--marketing-spotlight-x", x.toFixed(2) + "%");
        panel.style.setProperty("--marketing-spotlight-y", y.toFixed(2) + "%");
      };

      panel.addEventListener("pointermove", updateSpotlight);
      panel.addEventListener("pointerleave", function () {
        panel.style.setProperty("--marketing-spotlight-x", "55%");
        panel.style.setProperty("--marketing-spotlight-y", "10%");
      });
    });
  }
})();
