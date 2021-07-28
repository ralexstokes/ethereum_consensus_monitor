(ns com.github.ralexstokes.navigation
  (:require
   [clojure.string :as str]))

(defn- push-hash [e]
  (.pushState js/history (clj->js {}) "" (-> e .-target .-hash)))

(defn install []
  (-> (js/$ "a[data-toggle=\"tab\"]")
      (.on "shown.bs.tab" push-hash)))

(defn restore-last-state []
  (let [hash (-> js/document .-location .-hash)]
    (when (not (= "" hash))
      (-> (js/$ (str ".nav a[href=\"" (str/replace hash #"tab_" "") "\"]"))
          (.tab "show")))))
