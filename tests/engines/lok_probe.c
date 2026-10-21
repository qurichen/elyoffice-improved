#define LOK_USE_UNSTABLE_API
#include <LibreOfficeKit/LibreOfficeKitInit.h>
#include <dispatch/dispatch.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define PX 256
#define SENTINEL 0xAB

static LibreOfficeKit *lok;
static const char *doc_url, *pdf_url;
static volatile int polls;

static void probe(void) {
    printf("version=%s\n", lok->pClass->getVersionInfo(lok));
    LibreOfficeKitDocument *doc = lok->pClass->documentLoad(lok, doc_url);
    if (!doc) {
        printf("load=fail %s\n", lok->pClass->getError(lok));
        exit(1);
    }
    doc->pClass->initializeForRendering(doc, NULL);
    long w = 0, h = 0;
    doc->pClass->getDocumentSize(doc, &w, &h);
    printf("twips=%ldx%ld\npolls=%d\n", w, h, polls);

    unsigned char *tile = malloc(PX * PX * 4);
    memset(tile, SENTINEL, PX * PX * 4);
    doc->pClass->paintTile(doc, tile, PX, PX, 0, 0, PX * 15, PX * 15);
    long painted = 0, ink = 0;
    for (long i = 0; i < PX * PX; i++) {
        unsigned char *p = tile + i * 4;
        if (p[0] != SENTINEL || p[1] != SENTINEL || p[2] != SENTINEL || p[3] != SENTINEL) painted++;
        if (p[3] > 0 && p[0] < 128 && p[1] < 128 && p[2] < 128) ink++;
    }
    printf("painted=%ld\nink=%ld\npdf=%d\n", painted, ink, doc->pClass->saveAs(doc, pdf_url, "pdf", NULL));
    fflush(stdout);
}

static int poll_cb(void *data, int timeout_us) {
    polls++;
    return 0;
}
static void wake_cb(void *data) {}

static void *post_probe(void *arg) {
    sleep(1);
    dispatch_async(dispatch_get_main_queue(), ^{
        probe();
        _exit(0);
    });
    return NULL;
}

int main(int argc, char **argv) {
    if (argc != 6) {
        fprintf(stderr, "usage: install profile-url doc-url pdf-url unipoll|threaded\n");
        return 2;
    }
    doc_url = argv[3];
    pdf_url = argv[4];
    int unipoll = strcmp(argv[5], "unipoll") == 0;
    if (unipoll) setenv("SAL_LOK_OPTIONS", "unipoll", 1);
    lok = lok_init_2(argv[1], argv[2]);
    if (!lok) {
        fprintf(stderr, "lok_init_2 failed\n");
        return 1;
    }
    if (!unipoll) {
        probe();
        return 0;
    }
    pthread_t thread;
    pthread_create(&thread, NULL, post_probe, NULL);
    lok->pClass->runLoop(lok, poll_cb, wake_cb, NULL);
    return 0;
}
