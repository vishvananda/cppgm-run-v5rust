// Loop load promotion's preheader reads (host O2+) only where nothing may
// have freed the memory since the access or the entry that proved it
// readable: a zero-trip loop after munmap of the object its pointer, or a
// reference parameter, names never reads it, so neither may the preheader.
#include <cstdio>
#include <sys/mman.h>

struct Node
{
	const char* name;
	long pad;
};

__attribute__((noinline)) long after_access(Node* nodes, int n)
{
	long s = nodes->name[0];   // a dominating read of nodes->name
	munmap(nodes, 4096);
	for (int i = 0; i < n; ++i)
		s += nodes->name[i];
	return s;
}

__attribute__((noinline)) long after_entry(Node& node, int n)
{
	long s = 1;
	munmap(&node, 4096);
	for (int i = 0; i < n; ++i)
		s += node.name[i];
	return s;
}

__attribute__((noinline)) long kept(Node& node, int n)
{
	long s = 0;
	for (int i = 0; i < n; ++i)
		s += node.name[i];   // promoted: nothing runs before the loop
	return s;
}

Node* fresh()
{
	void* m = mmap(nullptr, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
	Node* node = static_cast<Node*>(m);
	node->name = "hello";
	return node;
}

int main(int argc, char**)
{
	const int none = argc - 1;
	std::printf("%ld\n", after_access(fresh(), none));
	std::printf("%ld\n", after_entry(*fresh(), none));
	Node* n = fresh();
	std::printf("%ld\n", kept(*n, none + 5));
	munmap(n, 4096);
	return 0;
}
