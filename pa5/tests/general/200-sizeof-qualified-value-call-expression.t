namespace N {
struct Tag {};
typedef char (&Yes)[1];
Yes probe(Tag);
}

int f()
{
  return sizeof(N::probe((N::Tag())));
}
