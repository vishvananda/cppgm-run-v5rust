all:
	sed '/^[[:space:]]*#/d' ../pa24.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
