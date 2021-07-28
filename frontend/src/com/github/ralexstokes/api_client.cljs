(ns com.github.ralexstokes.api-client
  (:require
   [cljs-http.client :as http]
   [cljs.core.async :as async]
   [camel-snake-kebab.core :as csk]
   [clojure.walk :as walk]))

(defn- url-with [path]
  (str "/api/v1" path))

(defn- kebab-keys [m]
  (into {}
        (map (fn [[k v]] [(csk/->kebab-case k) v]) m)))

(defn- kebab-keys-if-map [data]
  (if (map? data)
    (kebab-keys data)
    data))

(defn- get-body-with-converted-keys [resp]
  (let [body (or (:body resp)
                 :nil)]
    (walk/postwalk kebab-keys-if-map body)))

(defn- get-api-data [path]
  (let [resp (-> path
                 url-with
                 (http/get {:with-credentials? false}))]
    (async/map get-body-with-converted-keys [resp])))

(defn fetch-network-config []
  (get-api-data "/network-config"))

(defn fetch-nodes []
  (get-api-data "/nodes"))

(defn fetch-chain-data []
  (get-api-data "/chain"))

(defn fetch-fork-choice []
  (get-api-data "/fork-choice"))

(defn fetch-participation []
  (get-api-data "/participation"))

(defn fetch-deposit-contract []
  (get-api-data "/deposit-contract"))

(defn fetch-weak-subjectivity []
  (get-api-data "/weak-subjectivity"))
