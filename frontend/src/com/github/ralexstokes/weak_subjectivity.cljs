(ns com.github.ralexstokes.weak-subjectivity
  (:require
   [com.github.ralexstokes.state :as state]
   [com.github.ralexstokes.ui :as ui]
   [com.github.ralexstokes.block-explorer :as explorer]
   [clojure.string :as str]))

(defn view [state]
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
