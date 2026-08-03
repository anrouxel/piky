project(
    'piky',
    'rust',
    version: '0.1.0',
    meson_version: '>= 1.11',
    license: 'EUPL-1.2'
)

gnome = import('gnome')
i18n = import('i18n')

base_id = 'eu.anrouxel.piky'

is_devel = get_option('profile') == 'development'
if is_devel
    profile = 'Devel'
    application_id = '@0@.@1@'.format(base_id, profile)
else
    profile = ''
    application_id = base_id
endif

bindir = get_option('prefix') / get_option('bindir')
localedir = get_option('prefix') / get_option('localedir')
datadir = get_option('prefix') / get_option('datadir')
pkgdatadir = datadir / meson.project_name()

subdir('data')
subdir('po')
subdir('src')

gnome.post_install(
    gtk_update_icon_cache: true,
    glib_compile_schemas: true,
    update_desktop_database: true,
)