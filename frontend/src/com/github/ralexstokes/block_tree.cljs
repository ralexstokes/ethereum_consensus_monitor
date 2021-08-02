(ns com.github.ralexstokes.block-tree
  (:require
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.state :as state]
   [com.github.ralexstokes.block-explorer :as explorer]
   [cljsjs.d3]))

(def max-coordinate 100)
(def vertical-scaling 0.25)

(defn- node->slot [node]
  (-> node
      .-data
      .-slot))

(defn- node->root [node]
  (-> node
      .-data
      .-root))

(defn- node->weight [node]
  (-> node
      .-data
      .-weight))

(defn node->label [total-weight node]
  (let [root (ui/humanize-hex (node->root node))
        weight (node->weight node)
        weight-fraction (if (zero? total-weight) 0 (/ weight total-weight))]
    (str root ", " (-> weight-fraction (* 100) (.toFixed 2)) "%")))

(defn- compute-slot-fill [slots-per-epoch slot]
  (if (zero? (mod slot slots-per-epoch))
    "#e9f5ec"
    (if (even? slot)
      "#e9ecf5"
      "#fff")))

(defn- compute-slot-text [slots-per-epoch slot]
  (if (zero? (mod slot slots-per-epoch))
    (str slot " (epoch " (quot slot slots-per-epoch) ")")
    slot))

(defn- ->rectangle-guide [slots-per-epoch slot-count row-height index slot]
  (let [rect-height (max (/ 70 slot-count) 0.1)]
    [:g {:key slot
         :transform (str "translate(0 " (* 100 (/ index slot-count)) ")")}
     [:rect {:fill (compute-slot-fill slots-per-epoch slot)
             :x 0
             :y 0
             :width "100%"
             :height (str rect-height "%")}]
     [:text {:text-anchor "start"
             :x 0
             :y (str (/ rect-height 2) "%")
             :fill "#6c757d"}
      (compute-slot-text slots-per-epoch slot)]]))

(defn- slot-guide [tree slots-per-epoch row-height]
  (let [max-slot (apply max (map node->slot (.leaves tree)))
        min-slot (node->slot tree)
        slot-count (inc (- max-slot min-slot))]
    [:g {:font-size "10%"}
     (map-indexed #(->rectangle-guide slots-per-epoch slot-count row-height %1 %2) (range max-slot min-slot -1))]))

(def vertical-link-fn (-> (js/d3.linkVertical)
                          (.x #(.-x %))
                          (.y #(.-y %))))

(defn- ->block-link [index link]
  [:path {:key index
          :d (vertical-link-fn link)}])

(defn canonical-node? [d]
  true
  #_(-> d
        .-data
        .-is-canonical))

(defn compute-node-fill [d]
  (if (canonical-node? d)
    "#eec643"
    "#555"))

(defn compute-node-stroke [d]
  (if-let [_ (.-children d)]
    ""
    (if (canonical-node? d)
      "#d5ad2a"
      "")))

(defn node->block-explorer-link [network node]
  (let [root (node->root node)]
    (explorer/link-to-block network root)))

(defn- ->block-node [network total-weight index node]
  (let [x (.-x node)
        y (.-y node)]
    [:g {:key index
         :transform (str "translate(" x " " y ")")
         :font-size "80%"}
     [:a {:href (node->block-explorer-link network node)}
      [:circle {:fill (compute-node-fill node)
                :stroke (compute-node-stroke node)
                :stroke-width 3
                :r "0.9em"}]
      [:text {:text-anchor "start"}
       (node->label total-weight node)]]]))

(defn- block-tree [tree network total-weight]
  [:g {:transform "translate(0% 100%) rotate(180)"}
   [:g {:fill "none"
        :stroke "#555"
        :stroke-opacity 0.4
        :stroke-width 1.5}
    (map-indexed #(->block-link %1 %2) (.links tree))]
   [:g
    (map-indexed #(->block-node network total-weight %1 %2) (.descendants tree))]])

(defn- render-tree-from-layout [network slots-per-epoch total-weight tree]
  (let [row-height 12 #_(* vertical-scaling (.-innerHeight js/window))]
    [:svg.svg-content-responsive {:viewBox (str "0 0 " max-coordinate " " max-coordinate)
                                  :preserveAspectRatio "xMidYMid meet"}
     [slot-guide tree slots-per-epoch row-height]
     [block-tree tree network total-weight]]))

(defn- ->block-tree [proto-array-data]
  proto-array-data)

(defn- compute-layout [tree]
  (let [hierarchy (-> tree
                      clj->js
                      js/d3.hierarchy)
        render-fn (-> (js/d3.tree)
                      (.size #js [max-coordinate max-coordinate]))]
    (render-fn hierarchy)))

(defn- derive-block-tree-svg-from [network slots-per-epoch proto-array-data]
  (let [block-tree (->block-tree proto-array-data)
        total-weight (:weight block-tree 0)]
    (if (seq block-tree)
      [render-tree-from-layout network slots-per-epoch total-weight (compute-layout block-tree)]
      [:svg])))

(defn view [state]
  (let [proto-array-data (:proto-array @state)
        network (state/->network @state)
        slots-per-epoch (state/->slots-per-epoch @state)
        svg (derive-block-tree-svg-from network slots-per-epoch proto-array-data)]
    [:div.card
     [:div.card-header
      "Block tree over last 4 epochs"]
     [:div.card-body
      [:div
       [:p
        [:small
         "NOTE: nodes are labeled with their block root. Percentages are amounts of stake attesting to a block relative to the finalized block."]]
       [:div.svg-container
        svg]]]]))
