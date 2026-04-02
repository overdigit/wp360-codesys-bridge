prefix = /usr

all: wp360-codesys-bridge

install: all
	install -d $(DESTDIR)$(prefix)/bin
	install wp360-codesys-bridge $(DESTDIR)$(prefix)/bin/

clean:
	-rm -r target

.PHONY: all install clean
