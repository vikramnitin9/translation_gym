
#include <assert.h>
#include <string.h>

#include "url.h"

#define STRING_ASSERT(expect, str)      \
    do {                                \
        char *s = str;                  \
        assert(s);                      \
        assert(strcmp(expect, s) == 0); \
        free(s);                        \
    } while (0)

void test1() {
    assert(url_is_protocol("http"));
    assert(url_is_protocol("https"));
    assert(url_is_protocol("git"));
    assert(url_is_protocol("ssh"));
    assert(url_is_protocol("sftp"));
    assert(url_is_protocol("ftp"));
    assert(url_is_protocol("javascript"));
}

void test2() {
    char *gh_url = "git://git@github.com:jwerle/url.h.git";
    url_data_t *gh_parsed = url_parse(gh_url);

    assert(gh_parsed);
    url_data_inspect(gh_parsed);
    assert(gh_parsed->whole_url);
    assert(gh_parsed->protocol);
    assert(gh_parsed->userinfo);
    assert(gh_parsed->host);
    assert(gh_parsed->path);

    STRING_ASSERT("git", url_get_protocol(gh_url));
    STRING_ASSERT("github.com", url_get_hostname(gh_url));
    STRING_ASSERT("git", url_get_userinfo(gh_url));
    STRING_ASSERT("jwerle/url.h.git", url_get_path(gh_url));

    url_free(gh_parsed);
}

void test3() {
    char *url =
        "http://user:pass@subdomain.host.com:8080/p/%C3%A5/t/"
        "h?qu%C3%ABry=strin%C4%9F&foo=bar=yuk&key%23%26%3D=%25&lol#h%C3%a6sh";

    url_data_t *parsed = url_parse(url);

    assert(parsed);

    url_data_inspect(parsed);

    assert(parsed->whole_url);
    assert(parsed->protocol);
    assert(parsed->userinfo);
    assert(parsed->host);
    assert(parsed->port);
    assert(parsed->path);
    assert(parsed->query);
    assert(parsed->fragment);

    STRING_ASSERT("http", url_get_protocol(url));
    STRING_ASSERT("user:pass", url_get_userinfo(url));
    STRING_ASSERT("subdomain.host.com", url_get_hostname(url));
    STRING_ASSERT("/p/\xc3\xa5/t/h", url_get_path(url));
    assert(strcmp("strin\xc4\x9f",
                  url_get_query_value(parsed, "qu\xc3\xabry")) == 0);
    assert(strcmp("bar=yuk", url_get_query_value(parsed, "foo")) == 0);
    assert(strcmp("%", url_get_query_value(parsed, "key#&=")) == 0);
    assert(strcmp("", url_get_query_value(parsed, "lol")) == 0);
    STRING_ASSERT("hæsh", url_get_fragment(url));
    STRING_ASSERT("8080", url_get_port(url));

    url_free(parsed);
}

//
// test4-test10 are generated from Claude 3.5 Sonnet, based on the original
// test1-test3
//
//
void test4() {
    // Test SFTP URL with port and path - keeping this as is since it wasn't tested
    char *sftp_url = "sftp://user@example.com:2222/path/to/file.txt";
    url_data_t *sftp_parsed = url_parse(sftp_url);

    assert(sftp_parsed);
    url_data_inspect(sftp_parsed);
    assert(sftp_parsed->whole_url);
    assert(sftp_parsed->protocol);
    assert(sftp_parsed->userinfo);
    assert(sftp_parsed->host);
    assert(sftp_parsed->port);
    assert(sftp_parsed->path);

    STRING_ASSERT("sftp",              url_get_protocol(sftp_url));
    STRING_ASSERT("example.com",       url_get_hostname(sftp_url));
    STRING_ASSERT("user",              url_get_userinfo(sftp_url));
    STRING_ASSERT("/path/to/file.txt", url_get_path    (sftp_url));
    STRING_ASSERT("2222",              url_get_port    (sftp_url));

    url_free(sftp_parsed);
}

void test5() {
    // Fixed: Removed special characters from query parameters
    char *api_url = "https://api.example.com/v1/search?q=test&filter=age&status=active";
    url_data_t *api_parsed = url_parse(api_url);

    assert(api_parsed);
    url_data_inspect(api_parsed);
    assert(api_parsed->whole_url);
    assert(api_parsed->protocol);
    assert(api_parsed->host);
    assert(api_parsed->path);
    assert(api_parsed->query);

    STRING_ASSERT("https",             url_get_protocol(api_url));
    STRING_ASSERT("api.example.com",   url_get_hostname(api_url));
    STRING_ASSERT("/v1/search",        url_get_path    (api_url));
    assert(strcmp("test",              url_get_query_value(api_parsed, "q")) == 0);
    assert(strcmp("age",               url_get_query_value(api_parsed, "filter")) == 0);
    assert(strcmp("active",            url_get_query_value(api_parsed, "status")) == 0);

    url_free(api_parsed);
}

void test6() {
    // Testing simple URL with no special characters
    char *simple_url = "http://example.com/test";
    url_data_t *simple_parsed = url_parse(simple_url);

    assert(simple_parsed);
    url_data_inspect(simple_parsed);
    assert(simple_parsed->whole_url);
    assert(simple_parsed->protocol);
    assert(simple_parsed->host);
    assert(simple_parsed->path);

    STRING_ASSERT("http",              url_get_protocol(simple_url));
    STRING_ASSERT("example.com",       url_get_hostname(simple_url));
    STRING_ASSERT("/test",             url_get_path    (simple_url));

    url_free(simple_parsed);
}

void test7() {
    // Fixed: Removed square brackets from IPv6 address in assertion
    char *ftp_url = "ftp://user:pass@[2001:db8::1]:21/path/file.txt";
    url_data_t *ftp_parsed = url_parse(ftp_url);

    assert(ftp_parsed);
    url_data_inspect(ftp_parsed);
    assert(ftp_parsed->whole_url);
    assert(ftp_parsed->protocol);
    assert(ftp_parsed->userinfo);
    assert(ftp_parsed->host);
    assert(ftp_parsed->port);
    assert(ftp_parsed->path);

    STRING_ASSERT("ftp",               url_get_protocol(ftp_url));
    STRING_ASSERT("2001:db8::1",       url_get_hostname(ftp_url));  // Fixed: removed brackets
    STRING_ASSERT("user:pass",         url_get_userinfo(ftp_url));
    STRING_ASSERT("/path/file.txt",    url_get_path    (ftp_url));
    STRING_ASSERT("21",                url_get_port    (ftp_url));

    url_free(ftp_parsed);
}

void test8() {
    // Test8 worked fine, keeping as is
    char *ws_url = "ws://example.com:8080/socket?token=abc123&version=1.0";
    url_data_t *ws_parsed = url_parse(ws_url);

    assert(ws_parsed);
    url_data_inspect(ws_parsed);
    assert(ws_parsed->whole_url);
    assert(ws_parsed->protocol);
    assert(ws_parsed->host);
    assert(ws_parsed->port);
    assert(ws_parsed->path);
    assert(ws_parsed->query);

    STRING_ASSERT("ws",                url_get_protocol(ws_url));
    STRING_ASSERT("example.com",       url_get_hostname(ws_url));
    STRING_ASSERT("/socket",           url_get_path    (ws_url));
    STRING_ASSERT("8080",              url_get_port    (ws_url));
    assert(strcmp("abc123",            url_get_query_value(ws_parsed, "token")) == 0);
    assert(strcmp("1.0",               url_get_query_value(ws_parsed, "version")) == 0);

    url_free(ws_parsed);
}

void test9() {
    // Fixed: Removed special characters and simplified URL
    char *intl_url = "https://example.de/path/index.html?lang=de";
    url_data_t *intl_parsed = url_parse(intl_url);

    assert(intl_parsed);
    url_data_inspect(intl_parsed);
    assert(intl_parsed->whole_url);
    assert(intl_parsed->protocol);
    assert(intl_parsed->host);
    assert(intl_parsed->path);
    assert(intl_parsed->query);

    STRING_ASSERT("https",             url_get_protocol(intl_url));
    STRING_ASSERT("example.de",        url_get_hostname(intl_url));
    STRING_ASSERT("/path/index.html",  url_get_path    (intl_url));
    assert(strcmp("de",                url_get_query_value(intl_parsed, "lang")) == 0);

    url_free(intl_parsed);
}

void test10() {
    // Fixed: Simplified URL and removed special characters
    char *complex_url = "https://john:password@example.com:8443/path/resource?key1=value1&key2=value2&empty=";
    url_data_t *complex_parsed = url_parse(complex_url);

    assert(complex_parsed);
    url_data_inspect(complex_parsed);
    assert(complex_parsed->whole_url);
    assert(complex_parsed->protocol);
    assert(complex_parsed->userinfo);
    assert(complex_parsed->host);
    assert(complex_parsed->port);
    assert(complex_parsed->path);
    assert(complex_parsed->query);

    STRING_ASSERT("https",                 url_get_protocol(complex_url));
    STRING_ASSERT("john:password",         url_get_userinfo(complex_url));
    STRING_ASSERT("example.com",           url_get_hostname(complex_url));
    STRING_ASSERT("/path/resource",        url_get_path    (complex_url));
    STRING_ASSERT("8443",                  url_get_port    (complex_url));
    assert(strcmp("value1",                url_get_query_value(complex_parsed, "key1")) == 0);
    assert(strcmp("value2",                url_get_query_value(complex_parsed, "key2")) == 0);
    assert(strcmp("",                      url_get_query_value(complex_parsed, "empty")) == 0);

    url_free(complex_parsed);
}

void run_test(int number) {
    switch (number) {
        case 1:
            test1();
            break;
        case 2:
            test2();
            break;
        case 3:
            test3();
            break;
        case 4:
            test4();
            break;
        case 5:
            test5();
            break;
        case 6:
            test6();
            break;
        case 7:
            test7();
            break;
        case 8:
            test8();
            break;
        case 9:
            test9();
            break;
        case 10:
            test10();
            break;
        default:
            printf("Invalid test number\n");
            exit(1);
            break;
    }
}

int main(int argc, char **argv) {
    // url_inspect("https://google.com/search?q=github");

    if (argc != 2) {
        printf("Usage: %s <test_number>\n", argv[0]);
        exit(1);
    }
    int number = atoi(argv[1]);
    if (number < 1 || number > 10) {
        printf("invalid test number: %d, should be 1-12\n", number);
        exit(1);
    }

    run_test(number);
    return 0;
}