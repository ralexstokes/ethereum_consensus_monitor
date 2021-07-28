(ns com.github.ralexstokes.nodes
  (:require
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.state :as state]
   [clojure.string :as str]
   [com.github.ralexstokes.block-explorer :as explorer]))

(defn- name-from [version]
  (-> version
      (str/split "/")
      first
      str/capitalize))

(defn node-view [index {:keys [execution-client version healthy syncing]}]
  [:tr {:key index}
   [:th {:scope :row}
    (name-from version)]
   (when execution-client
     [:th {:scope :row}
      execution-client])
   [:td version]
   [:td {:style {:text-align "center"}}
    (if healthy
      ui/good-emoji
      ui/bad-emoji)]
   [:td {:style {:text-align "center"}}
    (if syncing
      "Yes"
      "No")]])

(defn view [state]
  (let [nodes (state/->nodes @state)
        has-execution-client? (not-any? nil? (map :execution-client nodes))]
    [:div#nodes-drawer.accordion
     [:div.card
      [:div.card-header
       [:button.btn.btn-link.btn-block.text-left {:type :button
                                                  :data-toggle "collapse"
                                                  :data-target "#collapseNodes"}
        "Nodes"]]
      [:div#collapseNodes.collapse.show {:data-parent "#nodes-drawer"}
       [:div.card-body
        [:table.table.table-hover
         [:thead
          [:tr
           [:th {:scope :col} "Consensus"]
           (when has-execution-client?
             [:th {:scope :col} "Execution"])
           [:th {:scope :col} "Version"]
           [:th {:scope :col
                 :style {:text-align "center"}} "Healthy?"]
           [:th {:scope :col
                 :style {:text-align "center"}} "Syncing?"]]]
         [:tbody
          (map-indexed node-view nodes)]]]]]]))

(defn head-view [network majority-root index {:keys [version execution-client head]}]
  (let [{:keys [slot root]} head]
    [:tr {:class (if (= root majority-root) :table-success :table-danger)
          :key index}
     [:th {:scope :row}
      (name-from version)]
     (when execution-client
       [:th {:scope :row}
        execution-client])
     [:td [:a {:href (explorer/link-to-slot network slot)} slot]]
     [:td [:a {:href (explorer/link-to-block network root)} (ui/humanize-hex root)]]]))

(defn compare-heads-view [state]
  (let [state @state
        nodes (state/->nodes state)
        network (state/->network state)
        majority-root (:majority-root state)
        has-execution-client? (not-any? nil? (map :execution-client nodes))]
    [:div.card
     [:div.card-header
      "Latest head by node"]
     [:div.card-body
      [:table.table.table-hover
       [:thead
        [:tr
         [:th {:scope :col} "Consensus"]
         (when has-execution-client?
           [:th {:scope :col} "Execution"])
         [:th {:scope :col} "Slot"]
         [:th {:scope :col} "Root"]]]
       [:tbody
        (map-indexed (partial head-view network majority-root) nodes)]]]]))
