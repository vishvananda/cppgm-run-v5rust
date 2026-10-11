// N3485 7.1.1p7-8 (its example `extern int d; static int d;`) and 7.5p5:
// redeclarations whose linkages (or language linkages) disagree are
// ill-formed; must be rejected
extern int d;
static int d;
int main() { return d; }
