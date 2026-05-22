prefix = /usr

all: wp360-codesys-bridge

install: all
	install -d $(DESTDIR)$(prefix)/bin
	install -d $(DESTDIR)$(prefix)/libexec/wp360-codesys-bridge
	install -d $(DESTDIR)$(prefix)/lib/systemd/system/codesyscontrol.service.d
	install wp360-codesys-bridge $(DESTDIR)$(prefix)/bin/
	install wp360-codesys-stopswitch $(DESTDIR)$(prefix)/libexec/wp360-codesys-bridge
	install 19-stopswitch.conf $(DESTDIR)$(prefix)/lib/systemd/system/codesyscontrol.service.d

clean:
	-rm -r target

.PHONY: all install clean
