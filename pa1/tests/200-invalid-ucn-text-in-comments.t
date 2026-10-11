// N3485 2.7, 2.2p1.3: comments are replaced by white space before tokens are
// formed; a \U or \u spelling inside a comment is not a
// universal-character-name (full program: exit 0)
// \UFFFFFFFF
/* \uD800 */
int main() { return 0; } // 0
