all:
	sed '/^[[:space:]]*#/d' ../pa13.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
