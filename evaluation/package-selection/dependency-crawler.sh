#!/bin/bash

SOURCE="$1"
RESULT="$2"

echo "Package,deps,reverse-deps" > "$RESULT"

for package in $(tr ',' ' ' < "$SOURCE"); do
  i=$(apt-rdepends "$package" 2>/dev/null | sed 's/^  Depends: //;s/ (.*)$//' | sort -u | wc -l)
  j=$(apt-rdepends -r "$package" 2>/dev/null | sed 's/^  Reverse Depends: //;s/ (.*)$//' | sort -u | wc -l)
  echo "$package,$i,$j" >> "$RESULT"
done

echo "Completed"