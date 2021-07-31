(ns com.github.ralexstokes.block-tree
  (:require
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.debug :as debug]
   [com.github.ralexstokes.state :as state]
   [com.github.ralexstokes.block-explorer :as explorer]
   [cljsjs.d3]))

;; (defn empty-svg! [svg]
;;   (.remove svg))

(defn node->label [total-weight d]
  (let [block (-> d .-data)
        root (-> block .-root ui/humanize-hex)
        weight (.-weight block)
        weight-fraction (if (zero? total-weight) 0 (/ weight total-weight))]
    (str root ", " (-> weight-fraction (* 100) (.toFixed 2)) "%")))

(defn canonical-node? [d]
  false)
  ;; (-> d
  ;;     .-data
  ;;     .-is_canonical))

(defn slot-guide->label [highest-slot offset]
  (let [slot (- highest-slot offset)]
    (if (zero? (mod slot 32))
      (str slot " (epoch " (quot slot 32) ")")
      slot)))

(defn node->y-offset [slot-offset dy node]
  (let [slot (-> node .-data .-slot)
        offset (- slot slot-offset)]
    (+ 0 (* dy offset) (/ dy 2))))

(defn- compute-fill [slot]
  (if (zero? (mod slot 32))
    "#e9f5ec"
    (if (even? slot)
      "#e9ecf5"
      "#fff")))

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

(defn node->block-explorer-link [network d]
  (let [root (-> d .-data  .-root (subs 2))]
    (explorer/link-to-block network root)))

;; (defn render-tree [network root width total-weight]
;;   (let [leaves (.leaves root)
;;         highest-slot (apply max (map #(-> % .-data  .-slot) leaves))
;;         lowest-slot (-> root .-data  .-slot)
;;         slot-count (- highest-slot lowest-slot)
;;         dy (.-dy root)
;;         height (* dy (inc slot-count))
;;         svg (-> (js/d3.selectAll "#fork-choice")
;;                 (.append "svg")
;;                 (.attr "viewBox" (array 0 0 width height))
;;                 (.attr "preserveAspectRatio" "xMinYMin meet")
;;                 (.attr "class" "svg-content-responsive"))
;;         background (-> svg
;;                        (.append "g")
;;                        (.attr "font-size" 10))
;;         slot-rects (-> background
;;                        (.append "g")
;;                        (.selectAll "g")
;;                        (.data (clj->js (into [] (range (inc slot-count)))))
;;                        (.join "g")
;;                        (.attr "transform" #(str "translate(0 " (* dy %) ")")))
;;         _ (-> slot-rects
;;               (.append "rect")
;;               (.attr "fill" #(compute-fill highest-slot %))
;;               (.attr "x" 0)
;;               (.attr "y" 0)
;;               (.attr "width" "100%")
;;               (.attr "height" dy))
;;         _ (-> slot-rects
;;               (.append "text")
;;               (.attr "text-anchor" "start")
;;               (.attr "y" (* dy 0.5))
;;               (.attr "x" 5)
;;               (.attr "fill" "#6c757d")
;;               (.text #(slot-guide->label highest-slot %)))
;;         g (-> svg
;;               (.append "g")
;;               (.attr "transform"
;;                      (str "translate(" (/ width 2) "," height ") rotate(180)")))
;;         _  (-> g
;;                (.append "g")
;;                (.attr "fill" "none")
;;                (.attr "stroke"  "#555")
;;                (.attr "stroke-opacity" 0.4)
;;                (.attr "stroke-width" 1.5)
;;                (.selectAll "path")
;;                (.data (.links root))
;;                (.join "path")
;;                (.attr "d" (-> (js/d3.linkVertical)
;;                               (.x #(.-x %))
;;                               (.y #(node->y-offset lowest-slot dy %)))))

;;         nodes   (-> g
;;                     (.append "g")
;;                     (.selectAll "g")
;;                     (.data (.descendants root))
;;                     (.join "g")
;;                     (.attr "transform" #(str "translate(" (.-x %) "," (node->y-offset lowest-slot dy %)  ")"))
;;                     (.append "a")
;;                     (.attr "href" (partial node->block-explorer-link network)))
;;         _ (-> nodes
;;               (.append "circle")
;;               (.attr "fill" compute-node-fill)
;;               (.attr "stroke" compute-node-stroke)
;;               (.attr "stroke-width" 3)
;;               (.attr "r" (* dy 0.2)))
;;         _ (-> nodes
;;               (.append "text")
;;               (.attr "dx" "1em")
;;               (.attr "transform" "rotate(180)")
;;               (.attr "text-anchor" "start")
;;               (.text (partial node->label total-weight)))]
;;     (.node svg)))

(defn- compute-layout [tree]
  (let [hierarchy (-> tree
                      clj->js
                      js/d3.hierarchy)]
    ((js/d3.tree) hierarchy)))

(defn- node->slot [node]
  (-> node
      .-data
      .-slot))

(defn- ->rectangle-guide [min-slot max-slot slot]
  (let [slot-count (- max-slot min-slot)]
    [:g {:key slot
         :transform (str "translate(0 " (* 0.0002 (- slot min-slot)) ")")}
     [:rect {:fill (compute-fill slot)
             :x 0
             :y 0
             :width "100%"
             :height "20px" #_(str (/ 1 slot-count) "%")}
      [:text {:font-size 10
              :text-anchor "start"
              :x 5
              :y 5
              :fill "#6c757d"}
       slot]]]))

(defn- slot-guide [tree]
  (let [max-slot (apply max (map node->slot (.leaves tree)))
        min-slot (node->slot tree)]
    [:g
     (map #(->rectangle-guide min-slot max-slot %1) (range min-slot max-slot))]))

;; background (-> svg
;;                (.append "g")
;;                (.attr "font-size" 10))
;; slot-rects (-> background
;;                (.append "g")
;;                (.selectAll "g")
;;                (.data (clj->js (into [] (range (inc slot-count)))))
;;                (.join "g")
;;                (.attr "transform" #(str "translate(0 " (* dy %) ")")))
;; _ (-> slot-rects
;;       (.append "text")
;;       (.attr "text-anchor" "start")
;;       (.attr "y" (* dy 0.5))
;;       (.attr "x" 5)
;;       (.attr "fill" "#6c757d")
;;       (.text #(slot-guide->label highest-slot %)))

(defn- render-tree-from-layout [network total-weight tree]
  [:svg.svg-content-responsive {:viewBox "0,0,200,200"
                                :preserveAspectRatio "xMinYMin meet"}
   [slot-guide tree]])

(defn view [state]
  (let [block-tree (:block-tree @state)
        network (state/->network @state)
        total-weight (:weight block-tree 0)
        svg (if (seq block-tree)
              [render-tree-from-layout network total-weight (compute-layout block-tree)]
              [:svg])]
    [:div.card
     [:div.card-header
      "Block tree over last 4 epochs"]
     [:div.card-body
      [:div#head-count-viewer
       [:p
        [:small
         "NOTE: nodes are labeled with their block root. Percentages are amounts of stake attesting to a block relative to the finalized block."]]
       [:div.svg-container
        svg]]]]))
