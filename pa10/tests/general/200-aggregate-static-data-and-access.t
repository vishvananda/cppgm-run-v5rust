struct Pair { int a; int b; };
struct Record { char tag; Pair pair; double weight; int tail[2]; };
const Record full = {1, {2, 3}, 4.5, {6}};
const Record empty = {};
int value = 9;
struct Alias { int& target; };
const Alias alias = {value};
union Overlay { int words[2]; double floating; };
Overlay overlay = {{4, 5}};
struct Access { volatile int first; int second; };
int read(Access* object) { return object->first + object->second; }
int quiet(const Pair& object) { return 0; }
int main() {
  overlay.words[0]++;
  return full.pair.b + full.tail[1] + empty.tag + alias.target + overlay.words[0];
}
