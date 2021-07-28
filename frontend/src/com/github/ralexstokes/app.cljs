(ns ^:figwheel-hooks com.github.ralexstokes.app
  (:require-macros [cljs.core.async.macros :refer [go]])
  (:require
   [com.github.ralexstokes.slot-clock :as clock]
   [com.github.ralexstokes.api-client :as api]
   [com.github.ralexstokes.block-tree :as block-tree]
   [com.github.ralexstokes.nodes :as nodes]
   [com.github.ralexstokes.navigation :as navigation]
   [com.github.ralexstokes.participation :as participation]
   [com.github.ralexstokes.block-explorer :as explorer]
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.state :as state]
   [cljsjs.d3]
   [clojure.string :as str]
   [reagent.core :as r]
   [reagent.dom :as r.dom]
   [cljs.core.async :refer [<! chan close!]]))

(def debug-mode? js/goog.DEBUG)
(def polling-frequency 3000) ;; ms
(def slot-clock-refresh-frequency 500) ;; ms

(defn- get-time []
  (.now js/Date))

(defn validator-info-view [state]
  (let [balance (state/->deposit-contract-balance state)]
    [:div.card
     [:div.card-header
      "Validator metrics"]
     [:div.card-body
      (when balance
        [:p "Balance in deposit contract: " (.toLocaleString balance) " ETH"])]]))

(defn ws-data-view [state]
  (let [state @state
        ws-data (state :ws-data)
        network (state/->network state)]
    [:div.card
     [:div.card-header
      "Weak subjectivity data (powered by " [:a {:href "https://github.com/adiasg/eth2-ws-provider"} "https://github.com/adiasg/eth2-ws-provider"] ")"]
     [:div.card-body
      (when-let [checkpoint (:checkpoint ws-data)]
        (let [root (-> checkpoint (str/split ":") first)
              stale? (:stale? ws-data)]
          [:div
           [:p "Latest checkpoint: " [:a {:href (explorer/link-to-block network root)} checkpoint]]
           [:p "Safe? (only use the checkpoint if safe!) " (if stale? ui/bad-emoji ui/good-emoji)]]))]]))

(defn container-row
  "layout for a 'widget'"
  [component]
  [:div.row.my-2
   [:div.col]
   [:div.col-10
    component]
   [:div.col]])

(defn nav-bar [state]
  (let [network (state/->network @state)]
    [:nav.navbar.navbar-expand-sm.navbar-light.bg-light
     [:a.navbar-brand {:href "#"} "eth monitor"]
     [:ul.nav.nav-pills.mr-auto
      [:li.nav-item
       [:a.nav-link.active {:data-toggle :tab
                            :href "#nav-tip-monitor"} "node monitor"]]
      [:li.nav-item
       [:a.nav-link {:data-toggle :tab
                     :href "#nav-block-tree"} "block tree"]]
      [:li.nav-item
       [:a.nav-link {:data-toggle :tab
                     :href "#nav-participation"} "participation"]]
      [:li.nav-item
       [:a.nav-link {:data-toggle :tab
                     :href "#nav-validator-info"} "validator info"]]
      [:li.nav-item
       [:a.nav-link {:data-toggle :tab
                     :href "#nav-ws-data"} "weak subjectivity"]]]
     [:div.ml-auto
      [:span.navbar-text (str "network: " network)]]]))

(defn app [state]
  [:div.container-fluid
   [nav-bar state]
   [:div.tab-content
    [container-row
     [clock/view state]]
    [:div#nav-tip-monitor.tab-pane.fade.show.active
     [container-row
      [nodes/view state]]
     [container-row
      [nodes/compare-heads-view state]]]
    [:div#nav-block-tree.tab-pane.fade.show
     [container-row
      [block-tree/view]]]
    [:div#nav-participation.tab-pane.fade.show
     [container-row
      [participation/view state]]]
    [:div#nav-validator-info.tab-pane.fade.show
     [container-row
      [validator-info-view state]]]
    [:div#nav-ws-data.tab-pane.fade.show
     [container-row
      [ws-data-view state]]]
    (when debug-mode?
      [container-row
       [ui/debug-view state]])]])

;; (defn refresh-fork-choice [state]
;;   (go (let [response (<! (api/fetch-fork-choice))
;;             block-tree (get-in response [:body :block_tree])]
;;         (when (seq (:root block-tree))
;;           (let [total-weight (:weight block-tree)
;;                 fork-choice (js/d3.hierarchy (clj->js block-tree))]
;;             (block-tree/render! (state/->network state) fork-choice total-weight))))))

(defn block-for [ms-delay]
  (let [c (chan)]
    (js/setTimeout (fn [] (close! c)) ms-delay)
    c))

;; (defn fetch-block-tree-if-new-head [state old new]
;;   (when (not= old new)
;;     (refresh-fork-choice state)))

(defn find-majority-root [nodes]
  (->> nodes
       (map (comp :root :head))
       frequencies
       (sort-by val >)
       first
       first))

(defn fetch-monitor-state [state]
  (go (let [nodes (<! (api/fetch-nodes))
            chain-data (<! (api/fetch-chain-data))
            majority-root (find-majority-root nodes)
            ;; old-root (get @state :majority-root "")
            ]
        (swap! state merge {:nodes nodes
                            :majority-root majority-root
                            :chain chain-data}))))
        ;; NOTE: we block here to give the backend time to compute
        ;; the updated fork choice... should be able to improve
        ;; (go (let [blocking-task (block-for 700)]
        ;;       (<! blocking-task)
        ;;       (fetch-block-tree-if-new-head old-root majority-root)))

(defn start-polling-for-heads [state]
  (let [polling-task (js/setInterval (partial fetch-monitor-state state) polling-frequency)]
    (swap! state assoc :polling-task polling-task)))

;; (defn fetch-participation-data [state]
;;   (go
;;     ;; NOTE: races update on server...
;;     ;; for now just delay a bit
;;     (let [blocking-task (block-for 1000)]
;;       (<! blocking-task)
;;       (let [response (<! (api/fetch-participation))
;;             data (get-in response [:body :data])]
;;         (swap! state assoc :participation-data data)))))

;; (defn fetch-deposit-contract-data [state]
;;   (go
;;     (let [response (<!
;;                     (api/fetch-deposit-contract))
;;           balance (get-in response [:body :balance])]
;;       (swap! state assoc :deposit-contract {:balance balance}))))

;; (defn start-polling-for-deposit-contract-data [state]
;;   (let [deposit-contract-polling-task (js/setInterval fetch-deposit-contract-data (* 3600 1000))]
;;     (swap! state assoc :deposit-contract-polling-task deposit-contract-polling-task)))

(defn update-slot-clock [network-config state]
  (let [old-epoch (state/->current-epoch @state)
        new-clock (clock/compute network-config (get-time))
        new-epoch (:epoch new-clock)]
    ;; (when (> new-epoch old-epoch)
    ;;   (fetch-participation-data state))
    (swap! state assoc :slot-clock new-clock)))

(defn start-slot-clock [network-config state]
  (let [timer-task (js/setInterval #(update-slot-clock network-config state) slot-clock-refresh-frequency)]
    (swap! state assoc :timer-task timer-task)))

;; (defn fetch-ws-data [state]
;;   (go
;;     (let [response (<! (api/fetch-weak-subjectivity))
;;           #_checkpoint #_(get-in response [:body :ws_checkpoint])
;;           stale? (get-in response [:body :is_safe])
;;           checkpoint "0x72c6cc7b697bee47458f8f9a3d90123e48e20b80c172e832ba9f4b8370548645:24333"]
;;       (swap! state assoc :ws-data {:checkpoint checkpoint :stale? stale?}))))

;; (defn refresh-ws-data [{:keys [seconds_per_slot slots_per_epoch]} state]
;;   (let [epoch-in-seconds (* seconds_per_slot slots_per_epoch)
;;         task (js/setInterval fetch-ws-data (* epoch-in-seconds 1000))]
;;     (swap! state assoc :ws-data-task task)))

(defn mount-app [state]
  (r.dom/render [app state] (js/document.getElementById "root")))

(defonce state (r/atom (state/new)))

(defn boot-app []
  (go
    (let [network-config (<! (api/fetch-network-config))]
      (swap! state merge {:network-config network-config
                          :slot-clock (clock/compute network-config (get-time))})
      (update-slot-clock network-config state)
      (fetch-monitor-state state)
      ;; (fetch-participation-data)
      ;; (fetch-deposit-contract-data)
      ;; (fetch-ws-data)
      (mount-app state)
      (navigation/install)
      (start-slot-clock network-config state)
      (start-polling-for-heads state)
      ;; (start-polling-for-deposit-contract-data)
      ;; (refresh-fork-choice)
      ;; (refresh-ws-data spec)
      (navigation/restore-last-state))))

(defonce init
  (boot-app))

;; for development
(defn ^:after-load re-render []
  (mount-app state))
