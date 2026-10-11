// The companion to 373: an ambiguity the program actually depends on is still
// an error.  Discarding a candidate is a substitution rule, not a licence to
// pick arbitrarily.

struct S { };
struct L { L(S); };
struct R { R(S); };

void operator<<(L, int);
void operator<<(R, int);

int main()
{
	S s;
	s << 1;
	return 0;
}
