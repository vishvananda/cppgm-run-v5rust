all:
	sed '/^[[:space:]]*#/d' ../pa11.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
