all:
	sed '/^[[:space:]]*#/d' ../pa14.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
