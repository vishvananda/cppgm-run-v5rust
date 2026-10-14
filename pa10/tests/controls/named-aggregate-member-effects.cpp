// expect exit 0
int count;
int next() { return ++count; }
int target;
struct Record { int first; int second; char text[4]; int& reference; volatile int status; };
int main() {
  Record object = {next(), next(), "abc", {target}, 3};
  object.reference = 4;
  object.status;
  typedef int Pair[2];
  Pair{next(), next()};
  return object.first == 1 && object.second == 2 && object.text[1] == 'b' && target == 4 && count == 4 ? 0 : 1;
}
