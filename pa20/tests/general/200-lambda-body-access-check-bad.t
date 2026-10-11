// N3485 11p2, 5.1.2p7: a lambda body at namespace scope has no access to X's
// private members; expect reject
class X { static int secret; };
int X::secret = 7;
int v = [] { return X::secret; }();
int main() { return v - 7; }
