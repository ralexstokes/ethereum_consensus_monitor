(ns com.github.ralexstokes.slot-clock
  (:require
   [com.github.ralexstokes.block-explorer :as explorer]
   [com.github.ralexstokes.state :as state]
   [com.github.ralexstokes.ui :as ui]))

(defn- in-seconds [time]
  (.floor js/Math
          (/ time 1000)))

(defn slot-from-timestamp [ts genesis-time seconds-per-slot]
  (quot (- ts genesis-time)
        seconds-per-slot))

(defn- calculate-eth2-time [current-time genesis-time seconds-per-slot slots-per-epoch]
  (let [time-in-secs (in-seconds current-time)
        slot (slot-from-timestamp time-in-secs genesis-time seconds-per-slot)
        slot-start-in-seconds  (+ genesis-time (* slot seconds-per-slot))
        delta (- time-in-secs slot-start-in-seconds)
        delta (if (< delta 0) (- seconds-per-slot (Math/abs delta)) delta)
        progress (* 100 (/ delta seconds-per-slot))]
    {:slot slot
     :epoch (Math/floor (/ slot slots-per-epoch))
     :slot-in-epoch (mod slot slots-per-epoch)
     :progress-into-slot progress}))

(defn compute [{:keys [genesis-time seconds-per-slot slots-per-epoch]} current-time]
  (calculate-eth2-time current-time genesis-time seconds-per-slot slots-per-epoch))

(defn- round-to-extremes [x]
  (let [margin 10]
    (cond
      (> x (- 100 margin)) 100
      :else x)))

(defn view [state]
  (let [state @state
        network (state/->network state)
        {:keys [slots-per-epoch]} (:network-config state)
        {:keys [slot epoch slot-in-epoch progress-into-slot]} (:slot-clock state)
        justified (state/->justified-checkpoint state)
        finalized (state/->finalized-checkpoint state)
        head-root (:majority-root state)
        link-to-epoch (explorer/link-to-epoch network epoch)
        link-to-slot (explorer/link-to-slot network slot)]
    [:div#chain-drawer.accordion
     [:div.card
      [:div.card-header
       [:button.btn.btn-link.btn-block.text-left {:type :button
                                                  :data-toggle "collapse"
                                                  :data-target "#collapseChain"}
        "Chain"]]
      [:div#collapseChain.collapse.show {:data-parent "#chain-drawer"}
       [:div.card-body
        [:div.mb-3
         "Epoch: " [:a {:href link-to-epoch} epoch]
         " (slot: " [:a {:href link-to-slot} slot] ")"]
        [:div.mb-3 (str "Slot in epoch: " slot-in-epoch " / " slots-per-epoch)]
        [:div.mb-3
         "Progress through slot:"
         [:div.progress
          [:div.progress-bar
           {:style
            {:width (str (round-to-extremes progress-into-slot) "%")}}]]]
        [:div.mb-3
         "Majority head root: "
         [:a {:href (explorer/link-to-block network head-root)} (ui/humanize-hex head-root)]]
        [:div.mb-3 "Justified checkpoint: epoch "
         [:a {:href (explorer/link-to-epoch network (:epoch justified))} (:epoch justified)]
         " with root "
         [:a {:href (explorer/link-to-block network (:root justified))} (-> justified :root ui/humanize-hex)]]
        [:div "Finalized checkpoint: epoch "
         [:a {:href (explorer/link-to-epoch network (:epoch finalized))} (:epoch finalized)]
         " with root "
         [:a {:href (explorer/link-to-block network (:root finalized))} (-> finalized :root ui/humanize-hex)]]]]]]))
