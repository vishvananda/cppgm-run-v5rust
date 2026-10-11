all:
	sed '/^[[:space:]]*#/d' ../pa23.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
