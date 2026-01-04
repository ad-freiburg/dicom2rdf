# dicom2rdf

`dicom2rdf` is a data pipeline that converts [DICOM](https://www.dicomstandard.org/)
SR documents to [RDF Turtle](https://www.w3.org/TR/turtle/) and makes them
searchable with [QLever](https://github.com/ad-freiburg/qlever), a high
performance graph database.

Unlike other approaches that only translate the bare hierarchical structure of a
DICOM SR document to RDF triples, `dicom2rdf` applies a second processing step
to transform this raw structure with a set of SPARQL CONSTRUCT queries. The
resulting schema yields simpler SPARQL queries that are both highly explorable
 and run faster.

## Example: Chronological [SSDE](https://radiopaedia.org/articles/size-specific-dose-estimate) by Acquisition Protocol

<img width="640" alt="Screenshot 2026-01-04 at 14 30 30" src="https://github.com/user-attachments/assets/fd509570-95d2-4637-be82-d3811063f2dd" />

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

4.  Start the pipeline:
    ```bash
    docker compose up --build
    ```

    1.  In the future, you may also start QLever and QLever UI without a full
        pipeline run:
        ```bash
        docker compose -f compose.yml -f compose.isolated.yml up qlever qlever-ui
        ```

5.  Wait for the "ready" message that displays the URLs of the QLever and QLever
    UI instances.

6.  Open the Qlever UI, start typing a subject, and explore the available predicates:

    <img width="640" alt="Screenshot 2026-01-04 at 14 32 12" src="https://github.com/user-attachments/assets/73814ee2-4677-4a46-8f09-2196a663a2c8" />

# License

TBD
