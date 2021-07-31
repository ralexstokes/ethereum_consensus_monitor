(ns com.github.ralexstokes.validator
  (:require
   [com.github.ralexstokes.state :as state]))

(defn view [state]
  (let [balance (state/->deposit-contract-balance state)]
    [:div.card
     [:div.card-header
      "Validator metrics"]
     [:div.card-body
      (when balance
        [:p "Balance in deposit contract: " (.toLocaleString balance) " ETH"])]]))
