// N3485 16.3.4p2: the painted K produced by K's own replacement is not
// replaced when F pastes its argument (full program: exit 7)
int xH = 5;
int K() { return 2; }
#define F(a) x ## a
#define K(a) a ( H + K
int main() { return K ( F ) ( ) ); } // 7
