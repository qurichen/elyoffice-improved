#include <mupdf/fitz.h>
#include <stdio.h>

int main(int argc, char **argv) {
    fz_context *ctx = fz_new_context(NULL, NULL, FZ_STORE_DEFAULT);
    if (argc != 2 || !ctx) return 2;
    fz_register_document_handlers(ctx);
    fz_try(ctx) {
        fz_document *doc = fz_open_document(ctx, argv[1]);
        fz_page *page = fz_load_page(ctx, doc, 0);
        fz_rect box = fz_bound_page(ctx, page);
        fz_pixmap *pix = fz_new_pixmap_from_page(ctx, page, fz_scale(2, 2), fz_device_rgb(ctx), 0);
        long dark = 0;
        for (long i = 0; i < (long)pix->w * pix->h; i++)
            if (pix->samples[i * pix->n] < 128) dark++;
        printf("pages=%d\nbounds=%.0fx%.0f\npixmap=%dx%d\ndark=%ld\n", fz_count_pages(ctx, doc),
               box.x1 - box.x0, box.y1 - box.y0, pix->w, pix->h, dark);
        fz_stext_page *text = fz_new_stext_page_from_page(ctx, page, NULL);
        fz_buffer *buf = fz_new_buffer(ctx, 256);
        fz_output *out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_page_as_text(ctx, out, text);
        fz_close_output(ctx, out);
        printf("text=%.*s\n", (int)fz_buffer_storage(ctx, buf, NULL), fz_string_from_buffer(ctx, buf));
    }
    fz_catch(ctx) {
        fprintf(stderr, "mupdf: %s\n", fz_caught_message(ctx));
        return 1;
    }
    return 0;
}
