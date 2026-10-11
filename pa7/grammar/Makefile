all:
	sed '/^[[:space:]]*#/d' ../pa7.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
