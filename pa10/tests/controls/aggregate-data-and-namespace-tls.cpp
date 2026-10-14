// expect exit 0
namespace {
  struct Pair { unsigned first; unsigned second; };
  struct Record { char tag; Pair pair; int tail[2]; };
  const Record record = {1, {2, 3}, {4}};
  extern thread_local int thread_value;
  int read_thread() { return thread_value; }
  thread_local int thread_value = 5;
}
int target = 6;
struct Alias { int& value; };
const Alias alias = {target};
union Overlay { int words[2]; long wide; };
Overlay overlay = {{7, 8}};
int main() {
  overlay.words[0]++;
  if (record.pair.second != 3 || record.tail[1] != 0) return 1;
  if (read_thread() != 5 || thread_value != 5) return 2;
  alias.value += 1;
  return target == 7 && overlay.words[0] == 8 ? 0 : 3;
}
