#include <streambuf>
class NullStreamBuffer : public std::streambuf {
protected:
  int overflow(int c) { return c; }
};
NullStreamBuffer g_buf;
std::streambuf* leak() { return &g_buf; }
int main() { return leak() != 0 ? 0 : 1; }
