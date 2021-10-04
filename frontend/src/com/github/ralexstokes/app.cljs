(ns ^:figwheel-hooks com.github.ralexstokes.app
  (:require-macros [cljs.core.async.macros :refer [go]])
  (:require
   [com.github.ralexstokes.slot-clock :as clock]
   [com.github.ralexstokes.api-client :as api]
  ;;  [com.github.ralexstokes.block-tree :as block-tree]
   [com.github.ralexstokes.nodes :as nodes]
   [com.github.ralexstokes.navigation :as navigation]
  ;;  [com.github.ralexstokes.validator :as validator]
  ;;  [com.github.ralexstokes.participation :as participation]
  ;;  [com.github.ralexstokes.weak-subjectivity :as weak-subjectivity]
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.state :as state]
   [reagent.core :as r]
   [reagent.dom :as r.dom]
   [cljs.core.async :refer [<! chan close!]]))

(def debug-mode? js/goog.DEBUG)
(def polling-frequency 1000) ;; ms
(def slot-clock-refresh-frequency 500) ;; ms
(defn now-ms []
  (.now js/Date))

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
     [:a.navbar-brand {:href "#"} "consensus monitor"]
     [:ul.nav.nav-pills.mr-auto
      [:li.nav-item
       [:a.nav-link.active {:data-toggle :tab
                            :href "#nav-tip-monitor"} "node monitor"]]
      ;; [:li.nav-item
      ;;  [:a.nav-link {:data-toggle :tab
      ;;                  :href "#nav-block-tree"} "block tree"]]
      ;; [:li.nav-item
      ;;  [:a.nav-link {:data-toggle :tab
      ;;                :href "#nav-participation"} "participation"]]
      ;; [:li.nav-item
      ;;  [:a.nav-link {:data-toggle :tab
      ;;                :href "#nav-validator-info"} "validator info"]]
      ;; [:li.nav-item
      ;;  [:a.nav-link {:data-toggle :tab
      ;;                :href "#nav-ws-data"} "weak subjectivity"]]
      ]
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
      [nodes/view state]]]
    ;; [:div#nav-block-tree.tab-pane.fade.show
    ;;  [container-row
    ;;   [block-tree/view state]]]
    ;; [:div#nav-participation.tab-pane.fade.show
    ;;  [container-row
    ;;   [participation/view state]]]
    ;; [:div#nav-validator-info.tab-pane.fade.show
    ;;  [container-row
    ;;   [validator/view state]]]
    ;; [:div#nav-ws-data.tab-pane.fade.show
    ;;  [container-row
    ;;   [weak-subjectivity/view state]]]
    (when debug-mode?
      [container-row
       [ui/debug-view state]])]])

(defn- merge-once [old new k]
  (let [has-data (seq (k old))]
    (if has-data
      old
      (assoc old k (k new)))))

(defn update-block-tree [state]
  (go (let [proto-array (<! (api/fetch-fork-choice))]
        (swap! state merge {:proto-array proto-array}))))

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

(defn- find-majority-root [nodes]
  (->> nodes
       (map (comp :root :head))
       frequencies
       (sort-by val >)
       first
       first))

(defn- on-new-nodes [state nodes]
  (let [old-majority-root (:majority-root @state)
        majority-root (find-majority-root nodes)
        new-root? (not= majority-root old-majority-root)]
    (swap! state merge {:nodes nodes
                        :majority-root majority-root
                        #_{:chain chain-data}})
    #_(when new-root?
        (update-block-tree state))))

(defn fetch-monitor-state [state]
  (go (let [nodes (<! (api/fetch-nodes))]
            ;; chain-data (<! (api/fetch-chain-data))
        (on-new-nodes state nodes))))

(defn start-polling-nodes [state]
  (let [polling-task (js/setInterval #(fetch-monitor-state state) polling-frequency)]
    (swap! state assoc :polling-task polling-task)))

(defn- update-slot-clock [state network-config]
  (swap! state assoc :slot-clock (clock/compute network-config (now-ms))))

(defn start-slot-clock [network-config state]
  (let [timer-task (js/setInterval #(update-slot-clock state network-config) slot-clock-refresh-frequency)]
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

(defn block-for [ms-delay]
  (let [c (chan)]
    (js/setTimeout (fn [] (close! c)) ms-delay)
    c))

(defn update-head [node id head]
  (if (= id (:id node))
    (assoc node :head head)
    node))

(defn- process-head-update [{:keys [id head]}]
  (let [nodes (state/->nodes @state)
        new-nodes (map #(update-head % id head) nodes)]
    (on-new-nodes state new-nodes)
    (swap! state assoc :nodes new-nodes)))

(defn- process-monitor-message [msg]
  (when-let [new-head (:new-head msg)]
    (process-head-update new-head)))

(defn connect-to-monitor []
  (let [conn (api/connect-stream process-monitor-message debug-mode?)]
    (swap! state assoc :monitor-conn conn)))

(defn boot-app []
  (go
    (let [network-config (<! (api/fetch-network-config))
          ;; ms-to-next-slot (clock/ms-to-next-slot network-config (now-ms))
          ]
      ;; load the network config before everything else...
      (swap! state assoc :network-config network-config)
      (update-slot-clock state network-config)
      (fetch-monitor-state state)
      (navigation/install)
      (mount-app state)
      (navigation/restore-last-state)
      (start-slot-clock network-config state)
      (connect-to-monitor)
      ;; (start-polling-nodes state)
      ;; block until next slot
      ;; (<! (block-for ms-to-next-slot))
      ;; (fetch-participation-data)
      ;; (fetch-deposit-contract-data)
      ;; (start-polling-for-deposit-contract-data)
      ;; (fetch-ws-data)
      ;; (refresh-ws-data spec)
      ;; (refresh-fork-choice)
      )))

(defonce init
  (boot-app))

;; for development
(defn ^:after-load re-render []
  (mount-app state))
