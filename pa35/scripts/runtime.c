/* runtime [-t SECONDS] [-m KBYTES] CMD ARGS...: runs CMD, then prints
   "user sys wall maxrss status" to stderr: times in seconds with
   microsecond resolution (wait4 rusage and a monotonic clock), peak RSS in
   KB, and status = CMD's exit code, "timeout" or "signal-N". -t kills CMD's
   process group after SECONDS of wall time; -m limits its address space.
   Built by the PA35 scripts with the host C compiler; /usr/bin/time's 10 ms
   resolution is too coarse for paired timing of sub-second compiles. */
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t timed_out;
static pid_t child;

static void on_alarm(int sig)
{
	(void)sig;
	timed_out = 1;
	kill(-child, SIGKILL);
}

int main(int argc, char **argv)
{
	struct timespec t0, t1;
	struct rusage ru;
	long seconds = 0, kbytes = 0;
	int status, i = 1;
	char state[32];
	while (i + 1 < argc && argv[i][0] == '-' && strlen(argv[i]) == 2) {
		if (argv[i][1] == 't')
			seconds = atol(argv[i + 1]);
		else if (argv[i][1] == 'm')
			kbytes = atol(argv[i + 1]);
		else
			break;
		i += 2;
	}
	if (i >= argc) {
		fprintf(stderr, "usage: runtime [-t SECONDS] [-m KBYTES] CMD ARGS...\n");
		return 2;
	}
	clock_gettime(CLOCK_MONOTONIC, &t0);
	child = fork();
	if (child < 0)
		return 2;
	if (child == 0) {
		setpgid(0, 0);
		if (kbytes > 0) {
			struct rlimit rl;
			rl.rlim_cur = rl.rlim_max = (rlim_t)kbytes * 1024;
			setrlimit(RLIMIT_AS, &rl);
		}
		execvp(argv[i], argv + i);
		_exit(127);
	}
	setpgid(child, child);
	if (seconds > 0) {
		signal(SIGALRM, on_alarm);
		alarm((unsigned)seconds);
	}
	while (wait4(child, &status, 0, &ru) < 0)
		;
	alarm(0);
	clock_gettime(CLOCK_MONOTONIC, &t1);
	if (timed_out)
		snprintf(state, sizeof state, "timeout");
	else if (WIFSIGNALED(status))
		snprintf(state, sizeof state, "signal-%d", WTERMSIG(status));
	else
		snprintf(state, sizeof state, "%d", WEXITSTATUS(status));
	fprintf(stderr, "%.6f %.6f %.6f %ld %s\n",
		ru.ru_utime.tv_sec + ru.ru_utime.tv_usec / 1e6,
		ru.ru_stime.tv_sec + ru.ru_stime.tv_usec / 1e6,
		(t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9,
		ru.ru_maxrss, state);
	if (timed_out)
		return 124;
	return WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
}
