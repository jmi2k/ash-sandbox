#!/bin/sh

for f in res/draw.vert res/draw.frag
do
    glslangValidator -V $f -o $f.spv
done
