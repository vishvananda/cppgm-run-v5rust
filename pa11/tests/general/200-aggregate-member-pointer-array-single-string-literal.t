// A one-element pointer array member from a braced string literal holds
// the literal's address (8.5.1p2) when the aggregate is initialized at
// run time; only a character array member takes a literal's code units.
struct Holder
{
  int tag;
  const char * const items[1];
};

int main(int argc, char **)
{
  Holder h = {argc, {"Jul"}};
  return h.items[0][0] == 'J' && h.items[0][2] == 'l' && h.tag == argc ? 0 : 1;
}
