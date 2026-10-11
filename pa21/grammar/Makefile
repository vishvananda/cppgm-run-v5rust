all:
	sed '/^[[:space:]]*#/d' ../pa21.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
