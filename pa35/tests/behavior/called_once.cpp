// Small functions with one call site that stay defined (host O2+: copied
// into that caller as well): an external member with a branch and a call, a
// function also reached through a pointer elsewhere (not copied), and one
// whose copy sits in a loop.
#include <cstdio>
#include <string>

struct Lexer
{
	std::string text;
	size_t pos = 0;
	unsigned long seen = 0;
	bool at_comment() const;
	void note(char c);
	unsigned long run();
};

bool Lexer::at_comment() const
{
	if (pos + 1 >= text.size() || text[pos] != '/')
		return false;
	return text[pos + 1] == '/' || text[pos + 1] == '*';
}

void Lexer::note(char c)
{
	if (c == '\n')
		seen = seen * 3 + 1;
	else
		seen += static_cast<unsigned char>(c);
}

unsigned long Lexer::run()
{
	unsigned long comments = 0;
	for (pos = 0; pos < text.size(); ++pos)
	{
		if (at_comment())
			++comments;
		note(text[pos]);
	}
	return comments * 1000 + seen;
}

int twice(int x) { return 2 * x + 1; }
int (*twice_pointer)(int) = twice;

int main()
{
	Lexer l;
	l.text = "a // b\n/* c */ d / e /x\n//";
	int t = 0;
	for (int i = 0; i < 10; ++i)
		t += twice_pointer(i);
	std::printf("%lu %d\n", l.run(), t);
	return 0;
}
