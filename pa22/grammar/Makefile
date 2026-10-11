all:
	sed '/^[[:space:]]*#/d' ../pa22.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
