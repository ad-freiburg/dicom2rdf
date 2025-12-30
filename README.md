# dicom2rdf

`dicom2rdf` is a data pipeline that
1.  converts large collections of DICOM SR documents to a raw RDF representation
2.  constructs semantic RDF from the raw RDF representation using information
    stored in DICOM data elements
3.  provides a SPARQL interface to query the data via [QLever](https://github.com/ad-freiburg/qlever),
    a high-performance graph database

<img width="1238" height="1668" alt="image" src="https://github.com/user-attachments/assets/37be7b1b-7d11-48d4-bd73-208db83dc186" />

# Prerequisites

- [Docker](https://docs.docker.com/get-docker/) with Docker Compose V2, or
- [Podman](https://podman.io/getting-started/installation) with `podman-compose`

# Usage
We use `docker` in the following examples, but `podman` works just as well.

1.  Clone this repository and `cd` into it:
    ```bash
    git clone https://github.com/ad-freiburg/dicom2rdf.git ~/dicom2rdf && cd ~/dicom2rdf
    ```

2.  Create empty bind mounts:
    ```bash
    mkdir -p data/{ttl-raw,ttl-semantic,index-raw,index-semantic}
    ```

3.  Create an `.env` file from the example and adjust as needed:
    ```bash
    cp .env.example .env
    ```

4.  Start the pipeline by providing both the base `compose.yml` as well as the
    `compose.pipeline.yml` to ensure correct execution order:
    ```bash
    docker compose -f compose.yml -f compose.pipeline.yml up --build
    ```

    1.  In the future, you may also start QLever and QLever UI without a full
        pipeline run:
        ```bash
        docker compose up qlever qlever-ui
        ```

5.  Wait for the "ready" message that displays the URLs of the QLever and QLever
    UI instances.

6.  Explore your DICOM SR documents using QLever UI. Example query:

```
# TOP 3 SR DOCUMENTS BY PROCEDURE COUNT
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX sct: <https://purl.bioontology.org/ontology/SNOMEDCT/>
SELECT (SAMPLE(?plabel) AS ?procedure_label) ?procedure_count WHERE {
  {
    SELECT DISTINCT ?procedure (COUNT(?procedure) AS ?procedure_count) WHERE {
      ?sr sct:71388002 ?procedure .
    }
    GROUP BY ?procedure
    ORDER BY DESC(?procedure_count)
    LIMIT 3
  }
  ?procedure rdfs:label ?plabel .
}
GROUP BY ?procedure_count
ORDER BY DESC(?procedure_count)
```

# License

TBD
