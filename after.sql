CREATE INDEX bagextract_adressen_28992_geometry
    ON adressen_28992
    USING gist (point);
