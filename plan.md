file server for storing rhythm game charts
each chart has a:
- chart set ID
- file
- numeric meta fields
    - chart constants, bpm, notecount, etc. go here
    - each numeric meta field is optional
- string meta fields? tbd
- tag groups
    - each chart can be tagged with at most one tag from a tag group

each chart set has attached metadata:
- song title
- song artist

how to handle chart formats that group many difficulties into one file?
- use some type of content-addressable layer? optional, i can just have
    each chart in the set link to the same file

support for 'pseudo-tags' where a tag group is computed from the values of some other
metadata fields?
    - i dont think this is needed for a minimal prototype, the pseudotag can just be computed
    from the actual tags

frontend will support:
- filter charts to only those with a tag (from a chosen tag group)
- search by name...?
- sort charts by metadata field
    - numeric fields are sorted... numerically
    - tag group fields are sorted based on the sort of the manifest

- title can be treated as a meta field. sort is alphabetical

ok so how do i represent all of this?

universes make it possible to store charts for multiple games (which likely have different
metadata fields) under one DB instance

- files are stored as URLs to the actual location. most likely this will point to some cloud bucket
- database tables:
    - one for universes: universe name
    - one for chart sets: universe | chart set ID | song title | song artist
        - there probably needs to be some way of extending the artist field, for things like
            collaborations, aliases, etc. but i'll leave that for later
    - one for charts: chart ID | chart set ID | designator | file | (json dump of metadata fields)
        - the designator is a textual label that distinguishes between charts in a chart set.
            this is almost always its difficulty. numeric level can be stored as a metadata field

- backend endpoints:
    - get chart file, given chart ID
    - get chart metadata
    - enumerate chart IDs of a chart set
    - create chart set
    - create chart, as part of a chart set
        - includes chart file upload
    - edit chart metadata
    - get list of charts
        - filter by metadata
