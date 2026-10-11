all:
	sed '/^[[:space:]]*#/d' ../pa25.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
