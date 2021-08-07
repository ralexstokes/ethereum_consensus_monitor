(ns com.github.ralexstokes.block-tree
  (:require
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.state :as state]
   [com.github.ralexstokes.block-explorer :as explorer]
   [cljsjs.d3]))

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

(defn- ->rectangle-guide [slots-per-epoch row-height index slot]
  [:g {:key slot
       :transform (str "translate(0 " (* row-height index) ")")}
   [:rect {:fill (compute-slot-fill slots-per-epoch slot)
           :x 0
           :y 0
           :width "100%"
           :height row-height}]
   [:text {:text-anchor "start"
           :alignment-baseline "middle"
           :x 0
           :y (/ row-height 2)
           :fill "#6c757d"}
    (compute-slot-text slots-per-epoch slot)]])

(defn- slot-guide [slots-per-epoch max-slot min-slot row-height]
  [:g
   (map-indexed #(->rectangle-guide slots-per-epoch row-height %1 %2) (range max-slot (dec min-slot) -1))])

(defn- ->slot-offset [node min-slot row-height]
  (let [slot (node->slot node)
        offset (- slot min-slot)
        dy (/ row-height 2)]
    (+ dy (* row-height offset))))


(defn- ->block-link [index vertical-link-fn link]
  [:path {:key index
          :d (vertical-link-fn link)}])

(defn canonical-node? [d]
  (-> d
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

(defn- ->block-node [network total-weight row-height min-slot index node]
  (let [x (.-x node)
        y (->slot-offset node min-slot row-height)]
    [:g {:key index
         :transform (str "translate(" x " " y ") rotate(180)")}
     [:a {:href (node->block-explorer-link network node)}
      [:circle {:fill (compute-node-fill node)
                :stroke (compute-node-stroke node)
                :stroke-width (* 0.1 row-height)
                :r (* 0.2 row-height)}]
      [:text {:text-anchor "start"
              :font-size "120%"
              :dx "1%"}
       (node->label total-weight node)]]]))

(defn mk-vertical-link-fn [min-slot row-height]
  (-> (js/d3.linkVertical)
      (.x #(.-x %))
      (.y #(->slot-offset % min-slot row-height))))

(defn- block-tree [tree network total-weight row-height min-slot max-height max-width]
  (let [vertical-link-fn (mk-vertical-link-fn min-slot row-height)]
    [:g {:transform (str "translate(" max-width " " max-height ") rotate(180)")}
     [:g {:fill "none"
          :stroke "#555"
          :stroke-opacity 0.4
          :stroke-width "0.08%"}
      (map-indexed #(->block-link %1 vertical-link-fn %2) (.links tree))]
     [:g
      (map-indexed #(->block-node network total-weight row-height min-slot %1 %2) (.descendants tree))]]))

(defn- render-tree-from-layout [network slots-per-epoch total-weight max-slot min-slot max-width max-height row-height tree]
  [:svg {:viewBox (str "0 0 " max-width " " max-height)
         :width "100%"
         :preserveAspectRatio "xMidYMid meet"
         :font-size "7%"}
   [slot-guide slots-per-epoch max-slot min-slot row-height]
   [block-tree tree network total-weight row-height min-slot max-height max-width]])

(defn- ->block-tree [proto-array-data]
  proto-array-data)

(defn- compute-layout [hierarchy w h]
  (let [render-fn (-> (js/d3.tree)
                      (.size #js [w h]))]
    (render-fn hierarchy)))

(defn- derive-block-tree-svg-from [network slots-per-epoch proto-array-data]
  (let [block-tree (->block-tree proto-array-data)
        total-weight (:weight block-tree 0)]
    (if (seq block-tree)
      (let [hierarchy (-> block-tree
                          clj->js
                          js/d3.hierarchy)
            max-slot (apply max (map node->slot (.leaves hierarchy)))
            min-slot (node->slot hierarchy)
            slot-count (inc (- max-slot min-slot))
            window-height (.-innerHeight js/window)
            scaling-factor 250
            row-height (/ window-height scaling-factor)
            max-width 100
            max-height (* slot-count row-height)]
        [render-tree-from-layout network slots-per-epoch total-weight max-slot min-slot max-width max-height row-height (compute-layout hierarchy max-width max-height)])
      [:svg])))

(defn view [state]
  (let [proto-array-data (:proto-array @state)
        network (state/->network @state)
        slots-per-epoch (state/->slots-per-epoch @state)
        svg (derive-block-tree-svg-from network slots-per-epoch proto-array-data)]
    [:div.card
     [:div.card-header
      "Block tree"]
     [:div.card-body
      [:p
       [:small
        "Percentages are amounts of stake attesting to a block relative to the finalized block."]]
      [:div svg]]]))
