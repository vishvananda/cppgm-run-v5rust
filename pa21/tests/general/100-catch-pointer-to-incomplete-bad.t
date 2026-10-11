// N3485 15.3p1: an exception-declaration shall not denote a pointer or
// reference to an incomplete type (other than cv void*); catch (Inc*)
// must be rejected
struct Inc;
void thrower();
int main() { try { thrower(); } catch (Inc* p) { return 1; } catch (...) { return 2; } return 0; }
