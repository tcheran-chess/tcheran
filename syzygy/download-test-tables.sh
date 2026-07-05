#!/bin/sh -e

(cd tables/chess && wget -w 1 -nc -i TEST-SOURCE.txt)
