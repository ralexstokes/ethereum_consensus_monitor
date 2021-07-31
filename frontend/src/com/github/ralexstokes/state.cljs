(ns com.github.ralexstokes.state)

(def zero-root  "0x0000000000000000000000000000000000000000000000000000000000000000")

(defn new []
  {:network-config {:network-name ""
                    :seconds-per-slot 12
                    :genesis-time 0
                    :slots-per-epoch 32}
   :slot-clock {:slot 0
                :epoch 0
                :slot-in-epoch 0
                :progress-into-slot 0}
   :nodes []
   :block-tree {}
   :chain {:justified-checkpoint {:epoch 0 :root zero-root} :finalized-checkpoint {:epoch 0 :root zero-root}}
   :majority-root zero-root
   :participation-data []
   :deposit-contract {:balance nil}
   :ws-data nil})

(defn ->network [state]
  (get-in state [:network-config :network-name]))

(defn ->deposit-contract-balance [state]
  (get-in state [:deposit-contract :balance]))

(defn ->current-epoch [state]
  (get-in state [:slot-clock :epoch]))

(defn ->nodes [state]
  (:nodes state))

(defn ->justified-checkpoint [state]
  (get-in state [:chain :justified-checkpoint]))

(defn ->finalized-checkpoint [state]
  (get-in state [:chain :finalized-checkpoint]))
