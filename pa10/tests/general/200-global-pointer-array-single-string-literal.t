// A one-element pointer array from a braced string literal holds the
// literal's address (8.5.1p2); only a character array takes a literal's
// code units (8.5.2p1).
static const char * const names[] = {"Jan"};
const char * const months[1] = {"Mar"};

const char *first_local()
{
  const char *local[] = {"May"};
  return local[0];
}

int main()
{
  return names[0][0] == 'J' && months[0][1] == 'a' && first_local()[2] == 'y' ? 0 : 1;
}
