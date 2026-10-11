all:
	sed '/^[[:space:]]*#/d' ../pa8.gram | ./gramparse

docker:
	./generate_html.sh

clean:
	-rm -f *.output *.out
