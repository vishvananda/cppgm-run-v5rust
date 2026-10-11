all:
	sed '/^[[:space:]]*#/d' ../pa6.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
