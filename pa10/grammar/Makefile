all:
	sed '/^[[:space:]]*#/d' ../pa10.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
