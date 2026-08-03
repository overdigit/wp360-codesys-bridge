prefix = /usr

all: wp360-codesys-bridge

install: all
	install -D wp360-codesys-bridge $(DESTDIR)$(prefix)/bin/wp360-codesys-bridge
	install -D wp360-codesys-stopswitch $(DESTDIR)$(prefix)/libexec/wp360-codesys-bridge/wp360-codesys-stopswitch
	install -D 10-codesys-root.conf $(DESTDIR)$(prefix)/lib/systemd/system/codesyscontrol.service.d/10-codesys-root.conf
	install -D 10-bridge-user.conf $(DESTDIR)$(prefix)/lib/systemd/system/wp360-codesys-ftp.service.d/10-bridge-user.conf
	install -D 10-bridge-user.conf $(DESTDIR)$(prefix)/lib/systemd/system/wp360-codesys-smtp.service.d/10-bridge-user.conf
	install -D 10-bridge-user.conf $(DESTDIR)$(prefix)/lib/systemd/system/wp360-codesys-bridge-rs.service.d/10-bridge-user.conf
	install -D 19-stopswitch.conf $(DESTDIR)$(prefix)/lib/systemd/system/codesyscontrol.service.d/19-stopswitch.conf
	install -D CODESYSControl_WP360.cfg $(DESTDIR)/etc/codesyscontrol/CODESYSControl_WP360.cfg

clean:
	-rm -r target

.PHONY: all install clean
