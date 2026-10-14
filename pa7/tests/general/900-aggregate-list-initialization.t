struct Pair { int a; int b; };
struct Record { char tag; Pair pair; double weight; int tail[2]; };
const Record full = {1, {2, 3}, 4.5, {6}};
const Record elided = {1, 2, 3, 4.5, 6};
const Record empty = {};
int value;
struct Alias { int& target; };
const Alias alias = {value};
int main() { return full.pair.b + elided.tail[1] + empty.tag + alias.target; }
