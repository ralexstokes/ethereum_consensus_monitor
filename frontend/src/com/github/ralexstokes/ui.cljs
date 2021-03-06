(ns com.github.ralexstokes.ui
  (:require
   [cljs.pprint :as pprint]
   [com.github.ralexstokes.state :as state]))

(def good-emoji "🟢")
(def bad-emoji "🔴")

(defn humanize-hex [hex-str]
  (let [hex-str (or hex-str state/zero-root)]
    (str (subs hex-str 2 6)
         ".."
         (subs hex-str (- (count hex-str) 4)))))

(defn render-edn [data]
  [:pre
   (with-out-str
     (pprint/pprint data))])

(defn debug-view [state]
  (let [state (assoc @state :proto-array :...elided)]
    [:div.row.debug
     (render-edn state)]))
