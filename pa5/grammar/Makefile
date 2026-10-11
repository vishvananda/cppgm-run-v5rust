all:
	sed '/^[[:space:]]*#/d' ../pa5.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
