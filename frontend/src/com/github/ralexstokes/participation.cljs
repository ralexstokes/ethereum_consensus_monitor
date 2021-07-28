(ns com.github.ralexstokes.participation)

(defn parse-rate [rate]
  (some-> rate
          js/parseFloat
          (.toFixed 2)))

(defn participation-view-for-epoch [index {:keys [epoch participation_rate justification_rate head_rate]}]
  (let [participation-rate (parse-rate participation_rate)
        justification-rate (parse-rate justification_rate)
        head-rate (parse-rate head_rate)]
    [:tr {:key index
          :class (if (>= justification-rate 66.6)
                   :table-warning
                   "")}
     [:th {:scope :row} (str "epoch " epoch)]
     [:td (str participation-rate "%")]
     [:td (str justification-rate "%")]
     [:td (if head-rate (str head-rate "%") "pending")]]))

(defn view [state]
  [:div#participation-view.card
   [:div.card-header
    "Participation metrics"]
   [:div.card-body
    [:div.card.bg-light
     [:div.card-header
      [:button.btn.btn-link {:data-toggle "collapse"
                             :data-target "#collapseParticipationLegend"}
       "Info"]]
     [:div#collapseParticipationLegend.collapse.show {:data-parent "#participation-view"}
      [:div.card-body
       [:p "Participation rate is percent of active stake that got an attestation on-chain."]
       [:p "Justification rate is percent of active stake that attested to the correct target. If this number is greater than 2/3, then the epoch is justified and colored golden."]
       [:p "Head rate is the precent of active validators who attested to the correct head."]]]]
    [:table.table.table-hover
     [:thead
      [:tr
       [:th {:scope :col} "Epoch"]
       [:th {:scope :col} "Participation rate"]
       [:th {:scope :col} "Justification rate"]
       [:th {:scope :col} "Head rate"]]]
     [:tbody
      (map-indexed #(participation-view-for-epoch %1 %2) (:participation-data @state))]]]])
