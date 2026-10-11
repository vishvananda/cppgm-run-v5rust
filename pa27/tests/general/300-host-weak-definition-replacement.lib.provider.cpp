int hook() { return 100; }
int weak_value = 800;
int unused_hook();
extern int weak_default;
int host_calls_unused() { return unused_hook(); }
int host_reads_default() { return weak_default; }
