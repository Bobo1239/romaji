#!/bin/env bash

# Source for igo.jar: https://osdn.net/projects/igo/

rm ipadic.zip
java -cp igo-0.4.5.jar net.reduls.igo.bin.BuildDic ipadic mecab/mecab-ipadic EUC-JP
zip -r9 ipadic.zip ipadic
rm -r ipadic
