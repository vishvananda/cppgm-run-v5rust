all:
	sed '/^[[:space:]]*#/d' ../pa20.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
