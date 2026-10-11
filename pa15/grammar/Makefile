all:
	sed '/^[[:space:]]*#/d' ../pa15.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
